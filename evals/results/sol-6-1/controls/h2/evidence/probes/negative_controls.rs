//! Bounded, in-memory regression probes for the seven control refutations.
use futures::StreamExt;
use h2_support::prelude::*;

#[tokio::test]
async fn decoy_1_cross_stream_continuation_is_rejected() {
    let mut codec = raw_codec! {
        read => [
            // HEADERS on stream 1 without END_HEADERS; indexed :method GET.
            0, 0, 1, 1, 0, 0, 0, 0, 1, 0x82,
            // CONTINUATION on stream 3, END_HEADERS, indexed :path /.
            0, 0, 1, 9, 4, 0, 0, 0, 3, 0x84,
        ];
    };
    let err = codec.next().await.unwrap().unwrap_err();
    assert!(matches!(err, h2::proto::Error::GoAway(_, Reason::PROTOCOL_ERROR, _)));
    println!("decoy-1: mismatched CONTINUATION rejected with connection PROTOCOL_ERROR");

    // Neighboring valid case: the same stream may complete the block.
    let mut codec = raw_codec! {
        read => [
            0, 0, 1, 1, 0, 0, 0, 0, 1, 0x82,
            0, 0, 1, 9, 4, 0, 0, 0, 1, 0x84,
        ];
    };
    let headers = poll_frame!(Headers, codec);
    assert_eq!(headers.stream_id(), 1);
    assert!(headers.is_end_headers());
    assert_closed!(codec);
}

#[tokio::test]
async fn decoy_2_over_release_is_rejected_without_changing_capacity() {
    let (io, mut srv) = mock::new();
    let server = async move {
        srv.assert_client_handshake().await;
        srv.recv_frame(frames::headers(1).request("GET", "https://example.com/").eos()).await;
        srv.send_frame(frames::headers(1).response(200)).await;
        srv.send_frame(frames::data(1, "hello")).await;
        // Dropping the unfinished response body cancels stream 1.
        srv.recv_frame(frame::Reset::new(StreamId::from(1), Reason::CANCEL)).await;
        srv.recv_frame(frames::go_away(0)).await;
        srv.recv_eof().await;
    };
    let client = async move {
        let (mut sender, mut conn) = client::handshake(io).await.unwrap();
        let response = sender.send_request(Request::get("https://example.com/").body(()).unwrap(), true).unwrap().0;
        let mut body = conn.drive(response).await.unwrap().into_body();
        let bytes = conn.drive(body.data()).await.unwrap().unwrap();
        assert_eq!(bytes.as_ref(), b"hello");
        let flow = body.flow_control();
        assert_eq!(flow.used_capacity(), 5);
        let available = flow.available_capacity();
        assert!(flow.release_capacity(6).is_err());
        assert_eq!(flow.used_capacity(), 5);
        assert_eq!(flow.available_capacity(), available);
        flow.release_capacity(5).unwrap();
        assert_eq!(flow.used_capacity(), 0);
        let available = flow.available_capacity();
        assert!(flow.release_capacity(1).is_err());
        assert_eq!(flow.used_capacity(), 0);
        assert_eq!(flow.available_capacity(), available);
        // Oversized usize input is also rejected before its u32 conversion.
        assert!(flow.release_capacity(usize::MAX).is_err());
        println!("decoy-2: releasing 6 of 5 bytes, releasing again, and usize::MAX all rejected; rejected calls preserve capacity");
        drop(body);
        drop(sender);
        conn.await.unwrap();
    };
    join(server, client).await;
}

#[tokio::test]
async fn false_1_pseudo_after_regular_header_is_rejected() {
    let mut codec = raw_codec! {
        read => [
            // Literal x: y followed by indexed :method GET, one HEADERS block.
            0, 0, 6, 1, 4, 0, 0, 0, 1,
            0, 1, b'x', 1, b'y', 0x82,
        ];
    };
    let err = codec.next().await.unwrap().unwrap_err();
    assert!(matches!(err, h2::proto::Error::Reset(id, Reason::PROTOCOL_ERROR, _) if id == 1));

    // The regular field in a previous fragment must also prevent a later pseudo.
    let mut codec = raw_codec! {
        read => [
            0, 0, 5, 1, 0, 0, 0, 0, 1, 0, 1, b'x', 1, b'y',
            0, 0, 1, 9, 4, 0, 0, 0, 1, 0x82,
        ];
    };
    let err = codec.next().await.unwrap().unwrap_err();
    assert!(matches!(err, h2::proto::Error::Reset(id, Reason::PROTOCOL_ERROR, _) if id == 1));
    println!("false-1: pseudo after regular header rejected with stream PROTOCOL_ERROR, within one frame and across CONTINUATION");
}

#[tokio::test]
async fn false_2_hpack_update_above_decoder_limit_is_rejected() {
    // HPACK size update: 31 + 98 + 31*128 = 4097, exceeding default 4096.
    let mut codec = raw_codec! {
        read => [0, 0, 4, 1, 4, 0, 0, 0, 1, 0x3f, 0xe2, 0x1f, 0x82,];
    };
    let err = codec.next().await.unwrap().unwrap_err();
    assert!(matches!(err, h2::proto::Error::GoAway(_, Reason::PROTOCOL_ERROR, _)));
    // Exactly 4096 is accepted (31 + 97 + 31*128).
    let mut codec = raw_codec! {
        read => [0, 0, 4, 1, 4, 0, 0, 0, 1, 0x3f, 0xe1, 0x1f, 0x82,];
    };
    let headers = poll_frame!(Headers, codec);
    assert_eq!(headers.into_parts().0.method, Some(Method::GET));
    assert_closed!(codec);
    println!("false-2: HPACK table size 4097 rejected before returning HEADERS; boundary 4096 accepted");
}

#[tokio::test]
async fn false_3_settings_max_frame_size_endpoints_are_accepted() {
    for size in [16_383u32, 16_384, 16_777_215, 16_777_216] {
        let mut payload = vec![0, 0, 6, 4, 0, 0, 0, 0, 0, 0, 5];
        payload.extend_from_slice(&size.to_be_bytes());
        let mut codec = Codec::new(mock_io::Builder::new().read(&payload).build());
        let result = codec.next().await.unwrap();
        if (16_384..=16_777_215).contains(&size) {
            match result.unwrap() {
                frame::Frame::Settings(settings) => assert_eq!(settings.max_frame_size(), Some(size)),
                other => panic!("unexpected frame: {:?}", other),
            }
            assert_closed!(codec);
        } else {
            assert!(matches!(result.unwrap_err(), h2::proto::Error::GoAway(_, Reason::PROTOCOL_ERROR, _)));
        }
    }
    println!("false-3: SETTINGS_MAX_FRAME_SIZE endpoints 16384 and 16777215 accepted; adjacent out-of-range values rejected");
}
