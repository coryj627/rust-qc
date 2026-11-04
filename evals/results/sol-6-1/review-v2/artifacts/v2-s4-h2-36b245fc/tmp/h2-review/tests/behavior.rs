use futures::StreamExt;
use h2_support::prelude::*;

#[tokio::test]
async fn reserve_large_capacity() {
    let (io, mut peer) = mock::new();
    let peer = async move {
        peer.assert_client_handshake().await;
        peer.recv_frame(frames::headers(1).request("POST", "https://example.com/"))
            .await;
        peer
    };
    let client = async move {
        let (mut client, mut conn) = client::handshake(io).await.unwrap();
        let request = Request::builder()
            .method("POST")
            .uri("https://example.com/")
            .body(())
            .unwrap();
        let (response, mut body) = client.send_request(request, false).unwrap();
        conn.drive(util::yield_once()).await;
        body.reserve_capacity(1);
        assert_eq!(body.capacity(), 1);
        body.reserve_capacity(1usize << 32);
        assert!(
            body.capacity() > 0,
            "a 4 GiB reservation must not cancel the reservation; got {}",
            body.capacity()
        );
        (client, conn, response, body)
    };
    let _ = join(peer, client).await;
}

#[tokio::test]
async fn push_promise_can_start_with_empty_fragment() {
    let bytes = [
        0, 0, 4, 5, 0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 6, 9, 4, 0, 0, 0, 1, 0x82, 0x87, 0x84, 0x41,
        1, b'x',
    ];
    let io = mock_io::Builder::new().read(&bytes).build();
    let mut codec = Codec::new(io);
    let frame = codec.next().await.unwrap();
    assert!(
        frame.is_ok(),
        "valid continued PUSH_PROMISE rejected: {frame:?}"
    );
}

#[tokio::test]
async fn invalid_trailers_are_rejected_locally() {
    let (io, _peer) = mock::new();
    let (mut client, _conn) = client::handshake(io).await.unwrap();
    let request = Request::builder()
        .uri("https://example.com/")
        .body(())
        .unwrap();
    let (_response, mut body) = client.send_request(request, false).unwrap();
    let mut trailers = HeaderMap::new();
    trailers.insert("connection", "close".parse().unwrap());
    assert!(
        body.send_trailers(trailers).is_err(),
        "connection-specific trailer accepted"
    );
}

#[tokio::test]
async fn push_promise_nonempty_fragment_controls() {
    for split in [1usize, 6] {
        let block = [0x82, 0x87, 0x84, 0x41, 1, b'x'];
        let flags = if split == block.len() { 4 } else { 0 };
        let mut wire = vec![0, 0, (4 + split) as u8, 5, flags, 0, 0, 0, 1, 0, 0, 0, 2];
        wire.extend_from_slice(&block[..split]);
        if split < block.len() {
            wire.extend_from_slice(&[0, 0, (block.len() - split) as u8, 9, 4, 0, 0, 0, 1]);
            wire.extend_from_slice(&block[split..]);
        }
        let io = mock_io::Builder::new().read(&wire).build();
        let mut codec = Codec::new(io);
        let frame = codec.next().await.unwrap().unwrap();
        assert!(matches!(frame, h2::frame::Frame::PushPromise(_)));
    }
}

#[tokio::test]
async fn header_list_exact_limit() {
    // GET + https + / + authority x: 42 + 44 + 38 + 43 = 167 octets.
    let wire = [0, 0, 6, 1, 5, 0, 0, 0, 1, 0x82, 0x87, 0x84, 0x41, 1, b'x'];
    for limit in [168usize, 166, 167] {
        let io = mock_io::Builder::new().read(&wire).build();
        let mut codec = Codec::new(io);
        codec.set_max_recv_header_list_size(limit);
        let frame = codec.next().await.unwrap().unwrap();
        let h2::frame::Frame::Headers(headers) = frame else {
            panic!("expected headers")
        };
        assert_eq!(
            headers.is_over_size(),
            167 > limit,
            "wrong classification for max_header_list_size={limit}"
        );
    }
}

#[tokio::test]
async fn accepted_invalid_trailers_reach_peer_as_protocol_error() {
    let (io, mut peer) = mock::new();
    let peer = async move {
        peer.assert_client_handshake().await;
        peer.recv_frame(frames::headers(1).request("GET", "https://example.com/"))
            .await;
        let err = peer.next().await.unwrap().unwrap_err();
        assert!(
            matches!(err, h2::proto::Error::Reset(_, Reason::PROTOCOL_ERROR, _)),
            "unexpected peer error: {err:?}"
        );
    };
    let client = async move {
        let (mut client, mut conn) = client::handshake(io).await.unwrap();
        let request = Request::builder()
            .uri("https://example.com/")
            .body(())
            .unwrap();
        let (response, mut body) = client.send_request(request, false).unwrap();
        let mut trailers = HeaderMap::new();
        trailers.insert("connection", "close".parse().unwrap());
        body.send_trailers(trailers).unwrap();
        let result = conn.drive(util::yield_once()).await;
        (client, conn, response, body, result)
    };
    let _ = join(peer, client).await;
}

#[tokio::test]
async fn header_limit_from_server_builder() {
    let (io, mut peer) = mock::new();
    let peer = async move {
        let settings = peer.assert_server_handshake().await;
        assert_eq!(settings.max_header_list_size(), Some(167));
        peer.send_frame(frames::headers(1).request("GET", "https://x/").eos())
            .await;
        let frame = peer.next().await.unwrap().unwrap();
        let h2::frame::Frame::Headers(headers) = frame else {
            panic!("expected response")
        };
        headers.into_parts().0.status.unwrap()
    };
    let server = async move {
        let mut conn = server::Builder::new()
            .max_header_list_size(167)
            .handshake::<_, Bytes>(io)
            .await
            .unwrap();
        let result = conn.accept().await;
        (conn, result)
    };
    match futures::future::select(Box::pin(peer), Box::pin(server)).await {
        futures::future::Either::Left((status, _)) => {
            panic!("request at the advertised limit was rejected with {status}")
        }
        futures::future::Either::Right(((_conn, result), _)) => assert!(result.unwrap().is_ok()),
    }
}
