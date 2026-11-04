use h2_support::prelude::*;

#[tokio::test]
async fn reservation_larger_than_u32_still_reserves_available_window() {
    let (io, _peer) = mock::new();
    let (mut client, _connection) = client::handshake(io).await.unwrap();
    let (_response, mut body) = client.send_request(Request::get("https://example.com/").body(()).unwrap(), false).unwrap();
    body.reserve_capacity(1usize << 32);
    assert!(body.capacity() > 0, "a nonzero reservation must not become a cancellation; actual capacity={}", body.capacity());
}

#[tokio::test]
async fn trailers_reject_connection_specific_headers() {
    let (io, _peer) = mock::new();
    let (mut client, _connection) = client::handshake(io).await.unwrap();
    let (_response, mut body) = client.send_request(Request::get("https://example.com/").body(()).unwrap(), false).unwrap();
    let mut trailers = HeaderMap::new();
    trailers.insert("connection", "close".parse().unwrap());
    let result = body.send_trailers(trailers);
    assert!(result.is_err(), "HTTP/2 trailers must reject connection-specific fields; actual={result:?}");
}

#[tokio::test]
async fn non_connect_authority_form_is_rejected() {
    let (io, _peer) = mock::new();
    let (mut client, _connection) = client::handshake(io).await.unwrap();
    let request = Request::builder().version(Version::HTTP_2).uri("example.com:443").body(()).unwrap();
    assert!(request.uri().scheme().is_none());
    assert!(request.uri().authority().is_some());
    let result = client.send_request(request, true);
    assert!(result.is_err(), "non-CONNECT request needs :scheme; actual={result:?}");
}

#[tokio::test]
async fn response_at_exact_header_list_limit_is_accepted() {
    let (io, mut peer) = mock::new();
    let peer_task = async move {
        peer.assert_client_handshake().await;
        peer.recv_frame(frames::headers(1).request("GET", "https://example.com/").eos()).await;
        peer.send_frame(frames::headers(1).response(200).eos()).await;
        idle_ms(20).await;
    };
    let client_task = async move {
        // The sole response field :status=200 occupies 7 + 3 + 32 = 42 octets.
        let (mut client, mut connection) = client::Builder::new().max_header_list_size(42).handshake::<_, Bytes>(io).await.unwrap();
        let (response, _body) = client.send_request(Request::get("https://example.com/").body(()).unwrap(), true).unwrap();
        let result = connection.drive(response).await;
        assert!(result.is_ok(), "a header list equal to its maximum is valid; actual={result:?}");
    };
    join(peer_task, client_task).await;
}

#[tokio::test]
async fn response_requires_status() {
    let (io, mut peer) = mock::new();
    let peer_task = async move {
        peer.assert_client_handshake().await;
        peer.recv_frame(frames::headers(1).request("GET", "https://example.com/").eos()).await;
        peer.send_frame(frames::headers(1).field("content-length", "0").eos()).await;
        idle_ms(20).await;
    };
    let client_task = async move {
        let (mut client, mut connection) = client::handshake(io).await.unwrap();
        let (response, _body) = client.send_request(Request::get("https://example.com/").body(()).unwrap(), true).unwrap();
        let result = connection.drive(response).await;
        assert!(result.is_err(), "a response without :status must error; actual={result:?}");
    };
    join(peer_task, client_task).await;
}
