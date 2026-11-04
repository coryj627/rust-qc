use h2_support::prelude::*;

#[tokio::test]
async fn oversized_trailers_are_not_silently_truncated() {
    let (io, mut srv) = mock::new();
    let peer = async move {
        srv.assert_client_handshake().await;
        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://example.com/")
                .eos(),
        )
        .await;
        srv.send_frame(frames::headers(1).response(200)).await;
        srv.send_frame(frames::headers(1).field("checksum", "a".repeat(100)).eos())
            .await;
        tokio::time::sleep(Duration::from_millis(30)).await;
    };
    let consumer = async move {
        let (mut client, conn) = client::Builder::new()
            .max_header_list_size(64)
            .handshake::<_, Bytes>(io)
            .await
            .unwrap();
        let driver = tokio::spawn(conn);
        let (response, _) = client
            .send_request(Request::get("https://example.com/").body(()).unwrap(), true)
            .unwrap();
        let mut body = response.await.unwrap().into_body();
        let result = body.trailers().await;
        driver.abort();
        result
    };
    let (_, result) = futures::future::join(peer, consumer).await;
    println!("oversized trailers: {result:?}");
    assert!(
        result.is_err(),
        "oversized trailers must report an error instead of silently dropping checksum metadata"
    );
}
