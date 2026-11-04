use h2_support::prelude::*;

#[tokio::test]
async fn accepts_header_list_at_advertised_limit() {
    let mut failed = Vec::new();
    for (limit, extra) in [(43u32, false), (42, false), (77, true), (76, true)] {
        let (io, mut srv) = mock::new();
        let peer = async move {
            let settings = srv.assert_client_handshake().await;
            assert_eq!(settings.max_header_list_size(), Some(limit));
            srv.recv_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;
            let response = frames::headers(1).response(200);
            let response = if extra {
                response.field("x", "a")
            } else {
                response
            };
            srv.send_frame(response.eos()).await;
            tokio::time::sleep(Duration::from_millis(10)).await;
        };
        let consumer = async move {
            let (mut client, conn) = client::Builder::new()
                .max_header_list_size(limit)
                .handshake::<_, Bytes>(io)
                .await
                .unwrap();
            let driver = tokio::spawn(conn);
            let (response, _) = client
                .send_request(Request::get("https://example.com/").body(()).unwrap(), true)
                .unwrap();
            let result = response.await;
            driver.abort();
            result
        };
        let (_, result) = futures::future::join(peer, consumer).await;
        println!("limit={limit}, extra={extra}: {result:?}");
        if result.is_err() {
            failed.push((limit, extra));
        }
    }
    assert!(
        failed.is_empty(),
        "header lists at or below advertised limit were rejected: {failed:?}"
    );
}
