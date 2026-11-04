use h2_support::prelude::*;
#[tokio::test]
async fn allow_empty_data_for_head() {
    let (io, mut srv) = mock::new();

    let srv = async move {
        let settings = srv.assert_client_handshake().await;
        assert_default_settings!(settings);
        srv.recv_frame(
            frames::headers(1)
                .request("HEAD", "https://example.com/")
                .eos(),
        )
        .await;
        srv.send_frame(
            frames::headers(1)
                .response(200)
                .field("content-length", 100),
        )
        .await;
        srv.send_frame(frames::data(1, "").eos()).await;
    };

    let h2 = async move {
        let (mut client, h2) = client::Builder::new()
            .handshake::<_, Bytes>(io)
            .await
            .unwrap();
        tokio::spawn(async {
            h2.await.expect("connection failed");
        });
        let request = Request::builder()
            .method(Method::HEAD)
            .uri("https://example.com/")
            .body(())
            .unwrap();
        let (response, _) = client.send_request(request, true).unwrap();
        let (_, mut body) = response.await.unwrap().into_parts();
        assert_eq!(body.data().await.unwrap().unwrap(), "");
    };

    join(srv, h2).await;
}

#[tokio::test]
async fn allow_empty_data_for_304() {
    let (io, mut srv) = mock::new();

    let srv = async move {
        let settings = srv.assert_client_handshake().await;
        assert_default_settings!(settings);
        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://example.com/")
                .eos(),
        )
        .await;
        srv.send_frame(
            frames::headers(1)
                .response(304)
                .field("content-length", 100),
        )
        .await;
        srv.send_frame(frames::data(1, "").eos()).await;
    };

    let h2 = async move {
        let (mut client, h2) = client::Builder::new()
            .handshake::<_, Bytes>(io)
            .await
            .unwrap();
        tokio::spawn(async {
            h2.await.expect("connection failed");
        });
        let request = Request::builder()
            .method(Method::GET)
            .uri("https://example.com/")
            .body(())
            .unwrap();
        let (response, _) = client.send_request(request, true).unwrap();
        let (_, mut body) = response.await.unwrap().into_parts();
        assert_eq!(body.data().await.unwrap().unwrap(), "");
    };

    join(srv, h2).await;
}

#[tokio::test]
async fn allow_headers_only_304() {
    let (io, mut srv) = mock::new();

    let srv = async move {
        let settings = srv.assert_client_handshake().await;
        assert_default_settings!(settings);
        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://example.com/")
                .eos(),
        )
        .await;
        srv.send_frame(
            frames::headers(1)
                .response(304)
                .field("content-length", 100).eos(),
        )
        .await;

    };

    let h2 = async move {
        let (mut client, h2) = client::Builder::new()
            .handshake::<_, Bytes>(io)
            .await
            .unwrap();
        tokio::spawn(async {
            h2.await.expect("connection failed");
        });
        let request = Request::builder()
            .method(Method::GET)
            .uri("https://example.com/")
            .body(())
            .unwrap();
        let (response, _) = client.send_request(request, true).unwrap();
        let (_, mut body) = response.await.unwrap().into_parts();
        assert!(body.data().await.is_none());
    };

    join(srv, h2).await;
}

