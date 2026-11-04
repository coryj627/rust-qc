use h2_support::prelude::*;

#[tokio::test]
async fn reserve_capacity_does_not_wrap() {
    let (io, _peer) = mock::new();
    let (mut client, _conn) = client::handshake(io).await.unwrap();
    let request = Request::get("https://example.com/").body(()).unwrap();
    let (_response, mut send) = client.send_request(request, false).unwrap();
    send.reserve_capacity(1);
    assert_eq!(send.capacity(), 1);
    send.reserve_capacity(1usize << 32);
    assert!(send.capacity() > 0, "positive reservation wrapped to zero");
}

async fn response_at_limit(limit: u32) -> Result<Response<RecvStream>, h2::Error> {
    let (io, mut peer) = mock::new();
    let peer_task = async move {
        let settings = peer.assert_client_handshake().await;
        assert_eq!(settings.max_header_list_size(), Some(limit));
        peer.recv_frame(frames::headers(1).request("GET", "https://example.com/").eos()).await;
        peer.send_frame(frames::headers(1).response(200).eos()).await;
        idle_ms(10).await;
    };
    let caller = async move {
        let (mut client, mut conn) = client::Builder::new()
            .max_header_list_size(limit).handshake::<_, Bytes>(io).await.unwrap();
        let (response, _) = client.send_request(Request::get("https://example.com/").body(()).unwrap(), true).unwrap();
        conn.drive(response).await
    };
    join(peer_task, caller).await.1
}

#[tokio::test]
async fn header_list_exact_limit_is_accepted() {
    // :status = 200 has 7 name + 3 value + 32 overhead = 42 bytes.
    assert!(response_at_limit(43).await.is_ok(), "control case");
    let response = response_at_limit(42).await;
    assert!(response.is_ok(), "exactly-at-limit response rejected: {:?}", response);
}

#[tokio::test]
async fn connection_headers_in_trailers_are_rejected() {
    let (io, _peer) = mock::new();
    let (mut client, _conn) = client::handshake(io).await.unwrap();
    let (_response, mut send) = client.send_request(Request::get("https://example.com/").body(()).unwrap(), false).unwrap();
    let mut trailers = HeaderMap::new();
    trailers.insert("connection", "close".parse().unwrap());
    assert!(send.send_trailers(trailers).is_err(), "invalid trailers accepted");
}

#[tokio::test]
async fn rejected_push_request_cleans_up_reserved_stream() {
    let (io, mut peer) = mock::new();
    let peer_task = async move {
        peer.assert_server_handshake().await;
        peer.send_frame(frames::headers(1).request("GET", "https://example.com/").eos()).await;
        idle_ms(50).await;
    };
    let caller = async move {
        let mut conn = server::handshake::<_>(io).await.unwrap();
        let (_request, mut respond) = conn.accept().await.unwrap().unwrap();
        for _ in 0..2 {
            let invalid_push = Request::post("https://example.com/push").body(()).unwrap();
            assert!(respond.push_request(invalid_push).is_err());
        }
        let debug = format!("{:?}", conn);
        let store_debug = debug.split("store: Store { slab: Slab { len: ").nth(1).unwrap();
        let stored: usize = store_debug.split(',').next().unwrap().parse().unwrap();
        assert_eq!(stored, 1, "only the original request should remain in the store after rejected pushes");
    };
    join(peer_task, caller).await;
}

#[tokio::test]
async fn accepted_invalid_trailers_are_rejected_by_peer() {
    tokio::time::timeout(Duration::from_secs(2), async {
        let (local, remote) = tokio::io::duplex(4096);
        let (mut client, driver) = client::handshake(local).await.unwrap();
        let driver = tokio::spawn(driver);
        let (_response, mut send) = client.send_request(
            Request::get("https://example.com/").body(()).unwrap(), false
        ).unwrap();
        let mut server = server::handshake(remote).await.unwrap();
        let (request, _respond) = server.accept().await.unwrap().unwrap();
        let mut body = request.into_body();
        let mut trailers = HeaderMap::new();
        trailers.insert("connection", "close".parse().unwrap());
        send.send_trailers(trailers).unwrap();
        let result = poll_fn(|cx| {
            let _ = server.poll_closed(cx);
            body.poll_data(cx)
        }).await;
        let error = result.unwrap().unwrap_err();
        assert_eq!(error.reason(), Some(Reason::PROTOCOL_ERROR));
        driver.abort();
    }).await.expect("local exchange timed out");
}
