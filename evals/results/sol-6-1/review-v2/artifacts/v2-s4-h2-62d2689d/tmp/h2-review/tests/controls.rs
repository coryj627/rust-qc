use h2_support::prelude::*;

#[tokio::test]
async fn small_reservation_assigns_capacity_and_zero_cancels_it() {
    let (io, _peer) = mock::new();
    let (mut client, _connection) = client::handshake(io).await.unwrap();
    let (_response, mut body) = client.send_request(Request::get("https://example.com/").body(()).unwrap(), false).unwrap();
    body.reserve_capacity(65535);
    assert_eq!(body.capacity(), 65535);
    body.reserve_capacity(0);
    assert_eq!(body.capacity(), 0);
}

#[tokio::test]
async fn regular_trailers_are_accepted_but_initial_connection_header_is_rejected() {
    let (io, _peer) = mock::new();
    let (mut client, _connection) = client::handshake(io).await.unwrap();
    let bad = Request::get("https://example.com/").header("connection", "close").body(()).unwrap();
    assert!(client.send_request(bad, false).is_err());
    let (_response, mut body) = client.send_request(Request::get("https://example.com/").body(()).unwrap(), false).unwrap();
    let mut trailers = HeaderMap::new();
    trailers.insert("x-result", "ok".parse().unwrap());
    assert!(body.send_trailers(trailers).is_ok());
}

#[tokio::test]
async fn absolute_uri_and_connect_authority_form_are_accepted() {
    let (io, _peer) = mock::new();
    let (mut client, _connection) = client::handshake(io).await.unwrap();
    assert!(client.send_request(Request::get("https://example.com/").body(()).unwrap(), true).is_ok());
    let request = Request::builder().method("CONNECT").version(Version::HTTP_2).uri("example.com:443").body(()).unwrap();
    assert!(client.send_request(request, true).is_ok());
}

#[tokio::test]
async fn response_one_octet_below_limit_is_accepted() {
    let (io, mut peer) = mock::new();
    let peer_task = async move {
        peer.assert_client_handshake().await;
        peer.recv_frame(frames::headers(1).request("GET", "https://example.com/").eos()).await;
        peer.send_frame(frames::headers(1).response(200).eos()).await;
        idle_ms(20).await;
    };
    let client_task = async move {
        let (mut client, mut connection) = client::Builder::new().max_header_list_size(43).handshake::<_, Bytes>(io).await.unwrap();
        let (response, _body) = client.send_request(Request::get("https://example.com/").body(()).unwrap(), true).unwrap();
        assert_eq!(connection.drive(response).await.unwrap().status(), StatusCode::OK);
    };
    join(peer_task, client_task).await;
}
