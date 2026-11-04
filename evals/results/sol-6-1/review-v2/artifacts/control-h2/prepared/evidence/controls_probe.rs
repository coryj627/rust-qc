// Ordinary functional probes for the unmodified h2 0.4.12 implementation.
// Run from /work/tmp/h2-controls with:
// cargo test --locked -p h2-tests --test controls_probe -- --nocapture --test-threads=1
use futures::{channel::oneshot, FutureExt, StreamExt};
use h2_support::prelude::*;

#[tokio::test]
async fn decoy_1_connect_omits_scheme_and_path() {
    let (io, mut srv) = mock::new();
    let peer = async move {
        srv.assert_client_handshake().await;
        srv.recv_frame(frames::headers(1).pseudo(frame::Pseudo {
            method: Some(Method::CONNECT),
            authority: Some(util::byte_str("tunnel.example.com:8443")),
            ..Default::default()
        }).eos()).await;
        srv.send_frame(frames::headers(1).response(200).eos()).await;
    };
    let client = async move {
        let (mut client, mut conn) = client::handshake(io).await.unwrap();
        let request = Request::builder().version(Version::HTTP_2)
            .method(Method::CONNECT).uri("https://tunnel.example.com:8443/")
            .body(()).unwrap();
        let (response, _) = client.send_request(request, true).unwrap();
        assert_eq!(conn.drive(response).await.unwrap().status(), StatusCode::OK);
    };
    join(peer, client).await;
    println!("decoy-1: wire pseudo-headers contain method and authority; scheme/path absent; 200 received");
}

#[tokio::test]
async fn decoy_2_overrelease_returns_error_without_changing_accounting() {
    let (io, mut srv) = mock::new();
    let peer = async move {
        srv.assert_client_handshake().await;
        srv.recv_frame(frames::headers(1).request("GET", "https://example.com/").eos()).await;
        srv.send_frame(frames::headers(1).response(200)).await;
        srv.send_frame(frames::data(1, "abc").eos()).await;
    };
    let client = async move {
        let (mut client, mut conn) = client::handshake(io).await.unwrap();
        let req = Request::builder().uri("https://example.com/").body(()).unwrap();
        let (response, _) = client.send_request(req, true).unwrap();
        let mut body = conn.drive(response).await.unwrap().into_body();
        assert_eq!(conn.drive(body.data()).await.unwrap().unwrap(), "abc");
        let fc = body.flow_control();
        assert_eq!(fc.used_capacity(), 3);
        let available = fc.available_capacity();
        let err = fc.release_capacity(4).expect_err("over-release must fail");
        assert_eq!(err.to_string(), "user error: release capacity too big");
        assert_eq!(fc.used_capacity(), 3);
        assert_eq!(fc.available_capacity(), available);
        fc.release_capacity(3).unwrap();
        assert_eq!(fc.used_capacity(), 0);
        assert!(body.data().await.is_none());
    };
    join(peer, client).await;
    println!("decoy-2: releasing 4 after receiving 3 fails; accounting unchanged; releasing 3 then succeeds");
}

#[tokio::test]
async fn false_1_trailers_survive_data_eof() {
    let (io, mut srv) = mock::new();
    let peer = async move {
        srv.assert_client_handshake().await;
        srv.recv_frame(frames::headers(1).request("GET", "https://example.com/").eos()).await;
        srv.send_frame(frames::headers(1).response(200)).await;
        let mut trailers = HeaderMap::new();
        trailers.insert("x-result", "ok".parse().unwrap());
        srv.send_frame(frame::Headers::trailers(1.into(), trailers)).await;
    };
    let client = async move {
        let (mut client, mut conn) = client::handshake(io).await.unwrap();
        let req = Request::builder().uri("https://example.com/").body(()).unwrap();
        let (response, _) = client.send_request(req, true).unwrap();
        let mut body = conn.drive(response).await.unwrap().into_body();
        assert!(conn.drive(body.data()).await.is_none());
        let trailers = body.trailers().await.unwrap().expect("trailers preserved");
        assert_eq!(trailers.len(), 1);
        assert_eq!(trailers["x-result"], "ok");
        assert!(body.trailers().await.unwrap().is_none());
    };
    join(peer, client).await;
    println!("false-1: data() returns None, then trailers() returns x-result: ok exactly once");
}

#[tokio::test]
async fn false_2_early_hints_do_not_resolve_response_future() {
    let (io, mut srv) = mock::new();
    let (hints_sent_tx, hints_sent_rx) = oneshot::channel();
    let (checked_tx, checked_rx) = oneshot::channel();
    let peer = async move {
        srv.assert_client_handshake().await;
        srv.recv_frame(frames::headers(1).request("GET", "https://example.com/").eos()).await;
        srv.send_frame(frames::headers(1).response(103).field("link", "</style.css>; rel=preload")).await;
        hints_sent_tx.send(()).unwrap();
        checked_rx.await.unwrap();
        srv.send_frame(frames::headers(1).response(200)).await;
        srv.send_frame(frames::data(1, "ok").eos()).await;
    };
    let client = async move {
        let (mut client, mut conn) = client::handshake(io).await.unwrap();
        let req = Request::builder().uri("https://example.com/").body(()).unwrap();
        let (mut response, _) = client.send_request(req, true).unwrap();
        conn.drive(hints_sent_rx).await.unwrap();
        conn.drive(util::yield_once()).await;
        assert!((&mut response).now_or_never().is_none(), "103 must not resolve response");
        checked_tx.send(()).unwrap();
        let response = conn.drive(response).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(!response.headers().contains_key("link"));
        let mut body = response.into_body();
        assert_eq!(conn.drive(body.data()).await.unwrap().unwrap(), "ok");
        assert!(body.data().await.is_none());
    };
    join(peer, client).await;
    println!("false-2: ResponseFuture remains Pending after 103 alone; then resolves to 200 with body ok");
}

#[tokio::test]
async fn false_3_zero_initial_window_holds_buffered_data() {
    let (io, mut srv) = mock::new();
    let (headers_seen_tx, headers_seen_rx) = oneshot::channel();
    let (checked_tx, checked_rx) = oneshot::channel();
    let peer = async move {
        srv.assert_client_handshake_with_settings(frames::settings().initial_window_size(0)).await;
        srv.recv_frame(frames::headers(1).request("POST", "https://example.com/")).await;
        headers_seen_tx.send(()).unwrap();
        checked_rx.await.unwrap();
        assert!(srv.next().now_or_never().is_none(), "no DATA at stream window zero");
        srv.send_frame(frames::settings().initial_window_size(3)).await;
        srv.recv_frame(frames::settings_ack()).await;
        srv.recv_frame(frames::data(1, "abc").eos()).await;
        srv.send_frame(frames::headers(1).response(204).eos()).await;
    };
    let client = async move {
        let (mut client, mut conn) = client::handshake(io).await.unwrap();
        conn.drive(util::yield_once()).await;
        let req = Request::builder().method(Method::POST)
            .uri("https://example.com/").body(()).unwrap();
        let (response, mut stream) = client.send_request(req, false).unwrap();
        stream.send_data(Bytes::from_static(b"abc"), true).unwrap();
        conn.drive(headers_seen_rx).await.unwrap();
        conn.drive(util::yield_once()).await;
        assert_eq!(stream.capacity(), 0);
        checked_tx.send(()).unwrap();
        assert_eq!(conn.drive(response).await.unwrap().status(), StatusCode::NO_CONTENT);
    };
    join(peer, client).await;
    println!("false-3: buffered 3-byte DATA absent at window 0; sent exactly after SETTINGS raises window to 3");
}

#[tokio::test]
async fn false_4_second_ping_is_rejected_and_first_completes() {
    let (io, mut srv) = mock::new();
    let peer = async move {
        srv.assert_client_handshake().await;
        for _ in 0..2 {
            srv.recv_frame(frames::ping(frame::Ping::USER)).await;
            srv.send_frame(frames::ping(frame::Ping::USER).pong()).await;
        }
        srv.recv_frame(frames::go_away(0)).await;
        srv.recv_eof().await;
    };
    let client = async move {
        let (client, mut conn) = client::handshake(io).await.unwrap();
        conn.drive(util::yield_once()).await;
        let mut pings = client::Connection::ping_pong(&mut conn).unwrap();
        pings.send_ping(Ping::opaque()).unwrap();
        let err = pings.send_ping(Ping::opaque()).expect_err("second ping must fail");
        assert_eq!(err.to_string(), "user error: send_ping before received previous pong");
        conn.drive(poll_fn(|cx| pings.poll_pong(cx))).await.unwrap();
        pings.send_ping(Ping::opaque()).unwrap();
        conn.drive(poll_fn(|cx| pings.poll_pong(cx))).await.unwrap();
        drop(client);
        conn.await.unwrap();
    };
    join(peer, client).await;
    println!("false-4: second pending send_ping returns user error; first pong completes and a subsequent ping succeeds");
}

#[tokio::test]
async fn false_5_unknown_setting_is_ignored_and_acked() {
    let (io, mut srv) = mock::new();
    let peer = async move {
        // One valid-length initial SETTINGS frame: unknown id 0x0abc, value 7.
        srv.send_bytes(&[0, 0, 6, 4, 0, 0, 0, 0, 0, 0x0a, 0xbc, 0, 0, 0, 7]).await;
        srv.read_preface().await.unwrap();
        srv.recv_frame(frames::settings()).await;
        srv.send_frame(frames::settings_ack()).await;
        srv.recv_frame(frames::settings_ack()).await;
        srv.recv_frame(frames::headers(1).request("GET", "https://example.com/").eos()).await;
        srv.send_frame(frames::headers(1).response(200).eos()).await;
    };
    let client = async move {
        let (mut client, mut conn) = client::handshake(io).await.unwrap();
        conn.drive(util::yield_once()).await;
        let req = Request::builder().uri("https://example.com/").body(()).unwrap();
        let (response, _) = client.send_request(req, true).unwrap();
        assert_eq!(conn.drive(response).await.unwrap().status(), StatusCode::OK);
    };
    join(peer, client).await;
    println!("false-5: SETTINGS id 0x0abc ignored and ACKed; subsequent GET completes with 200");
}
