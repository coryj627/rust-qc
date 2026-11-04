use h2_support::prelude::*;

#[tokio::test]
async fn missing_response_status_is_an_error() {
    let (io, mut srv) = mock::new();
    let peer = async move {
        srv.assert_client_handshake().await;
        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://example.com/")
                .eos(),
        )
        .await;
        // No :status pseudo-header is present.
        srv.send_frame(frames::headers(1).field("content-length", "0").eos())
            .await;
        tokio::time::sleep(Duration::from_millis(20)).await;
    };
    let consumer = async move {
        let (mut client, conn) = client::handshake(io).await.unwrap();
        let driver = tokio::spawn(conn);
        let (response, _) = client
            .send_request(Request::get("https://example.com/").body(()).unwrap(), true)
            .unwrap();
        let result = response.await;
        driver.abort();
        result
    };
    let (_, result) = futures::future::join(peer, consumer).await;
    println!("response with no :status: {result:?}");
    assert!(
        result.is_err(),
        "a response without mandatory :status cannot be reported as success"
    );
}
