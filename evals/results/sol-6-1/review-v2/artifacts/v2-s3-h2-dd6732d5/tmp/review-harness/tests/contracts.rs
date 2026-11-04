use h2_support::prelude::*;

#[tokio::test]
async fn pushed_head_accepts_representation_content_length() {
    let (io, mut srv) = mock::new();
    let remote = async move {
        srv.assert_client_handshake().await;
        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://example.com/")
                .eos(),
        )
        .await;
        srv.send_frame(frames::push_promise(1, 2).request("HEAD", "https://example.com/item"))
            .await;
        srv.send_frame(frames::headers(1).response(200).eos()).await;
        srv.send_frame(
            frames::headers(2)
                .response(200)
                .field("content-length", 100)
                .eos(),
        )
        .await;
        idle_ms(10).await;
    };
    let local = async move {
        let (mut client, mut conn) = client::handshake(io).await.unwrap();
        let (mut response, _) = client
            .send_request(Request::get("https://example.com/").body(()).unwrap(), true)
            .unwrap();
        let work = async move {
            let promise = response
                .push_promises()
                .push_promise()
                .await
                .unwrap()
                .unwrap();
            let (req, pushed_response) = promise.into_parts();
            assert_eq!(req.method(), Method::HEAD);
            let result = pushed_response.await;
            assert!(
                result.is_ok(),
                "valid pushed HEAD response rejected: {result:?}"
            );
        };
        conn.drive(work).await;
    };
    join(remote, local).await;
}

#[tokio::test]
async fn not_modified_accepts_empty_end_data() {
    let (io, mut srv) = mock::new();
    let remote = async move {
        srv.assert_client_handshake().await;
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
        idle_ms(10).await;
    };
    let local = async move {
        let (mut client, mut conn) = client::handshake(io).await.unwrap();
        let (response, _) = client
            .send_request(Request::get("https://example.com/").body(()).unwrap(), true)
            .unwrap();
        let work = async move {
            let mut body = response.await.unwrap().into_body();
            let result = body.data().await;
            assert!(
                matches!(result, Some(Ok(ref data)) if data.is_empty()),
                "empty 304 body rejected: {result:?}"
            );
        };
        conn.drive(work).await;
    };
    join(remote, local).await;
}

#[tokio::test]
async fn reserve_large_body_does_not_cancel_reservation() {
    let (io, _remote) = mock::new();
    let (mut client, _conn) = client::handshake(io).await.unwrap();
    let (_, mut body) = client
        .send_request(
            Request::post("https://example.com/").body(()).unwrap(),
            false,
        )
        .unwrap();
    body.reserve_capacity(100);
    assert!(body.capacity() > 0, "control reservation should succeed");
    body.reserve_capacity(1usize << 32);
    assert!(
        body.capacity() > 0,
        "4 GiB reservation silently became zero"
    );
}

#[tokio::test]
async fn pushed_get_accepts_early_hints_before_final_response() {
    let (io, mut srv) = mock::new();
    let remote = async move {
        srv.assert_client_handshake().await;
        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://example.com/")
                .eos(),
        )
        .await;
        srv.send_frame(frames::push_promise(1, 2).request("GET", "https://example.com/item"))
            .await;
        srv.send_frame(frames::headers(2).response(103)).await;
        srv.send_frame(frames::headers(2).response(200).eos()).await;
        srv.send_frame(frames::headers(1).response(200).eos()).await;
        idle_ms(10).await;
    };
    let local = async move {
        let (mut client, mut conn) = client::handshake(io).await.unwrap();
        let (mut response, _) = client
            .send_request(Request::get("https://example.com/").body(()).unwrap(), true)
            .unwrap();
        let work = async move {
            let promise = response
                .push_promises()
                .push_promise()
                .await
                .unwrap()
                .unwrap();
            let (_, pushed_response) = promise.into_parts();
            assert_eq!(pushed_response.await.unwrap().status(), StatusCode::OK);
        };
        conn.drive(work).await;
    };
    join(remote, local).await;
}

#[tokio::test]
async fn padded_data_returns_discarded_padding_capacity() {
    use futures::StreamExt;
    let (io, mut srv) = mock::new();
    let remote = async move {
        let settings = srv.assert_client_handshake().await;
        assert_eq!(settings.initial_window_size(), Some(16));
        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://example.com/")
                .eos(),
        )
        .await;
        srv.send_frame(frames::headers(1).response(200)).await;
        // Sixteen flow-controlled octets: pad length, one body octet, fourteen padding octets.
        let mut padded = vec![0, 0, 16, 0, 8, 0, 0, 0, 1, 14, b'x'];
        padded.extend([0; 14]);
        srv.send_bytes(&padded).await;
        let result = tokio::time::timeout(Duration::from_millis(100), srv.next()).await;
        assert!(
            result.is_ok(),
            "peer exhausted its 16-byte window but no WINDOW_UPDATE arrived after body consumption"
        );
        let received = result.unwrap().unwrap().unwrap();
        assert!(
            matches!(received, h2::frame::Frame::WindowUpdate(_)),
            "expected WINDOW_UPDATE, got {received:?}"
        );
        srv.send_frame(frames::data(1, "y").eos()).await;
    };
    let local = async move {
        let (mut client, mut conn) = client::Builder::new()
            .initial_window_size(16)
            .handshake::<_, Bytes>(io)
            .await
            .unwrap();
        let (response, _) = client
            .send_request(Request::get("https://example.com/").body(()).unwrap(), true)
            .unwrap();
        let work = async move {
            let mut body = response.await.unwrap().into_body();
            let data = body.data().await.unwrap().unwrap();
            assert_eq!(data, "x");
            body.flow_control().release_capacity(data.len()).unwrap();
            assert_eq!(body.data().await.unwrap().unwrap(), "y");
        };
        conn.drive(work).await;
    };
    join(remote, local).await;
}

#[tokio::test]
async fn control_pushed_head_with_zero_content_length() {
    let (io, mut srv) = mock::new();
    let remote = async move {
        srv.assert_client_handshake().await;
        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://example.com/")
                .eos(),
        )
        .await;
        srv.send_frame(frames::push_promise(1, 2).request("HEAD", "https://example.com/item"))
            .await;
        srv.send_frame(frames::headers(1).response(200).eos()).await;
        srv.send_frame(
            frames::headers(2)
                .response(200)
                .field("content-length", 0)
                .eos(),
        )
        .await;
        idle_ms(10).await;
    };
    let local = async move {
        let (mut client, mut conn) = client::handshake(io).await.unwrap();
        let (mut response, _) = client
            .send_request(Request::get("https://example.com/").body(()).unwrap(), true)
            .unwrap();
        let work = async move {
            let promise = response
                .push_promises()
                .push_promise()
                .await
                .unwrap()
                .unwrap();
            let (req, pushed_response) = promise.into_parts();
            assert_eq!(req.method(), Method::HEAD);
            let result = pushed_response.await;
            assert!(
                result.is_ok(),
                "valid pushed HEAD response rejected: {result:?}"
            );
        };
        conn.drive(work).await;
    };
    join(remote, local).await;
}

#[tokio::test]
async fn control_not_modified_with_headers_end_stream() {
    let (io, mut srv) = mock::new();
    let remote = async move {
        srv.assert_client_handshake().await;
        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://example.com/")
                .eos(),
        )
        .await;
        srv.send_frame(
            frames::headers(1)
                .response(304)
                .field("content-length", 100)
                .eos(),
        )
        .await;
        idle_ms(10).await;
    };
    let local = async move {
        let (mut client, mut conn) = client::handshake(io).await.unwrap();
        let (response, _) = client
            .send_request(Request::get("https://example.com/").body(()).unwrap(), true)
            .unwrap();
        let work = async move {
            let mut body = response.await.unwrap().into_body();
            let result = body.data().await;
            assert!(result.is_none(), "empty 304 body rejected: {result:?}");
        };
        conn.drive(work).await;
    };
    join(remote, local).await;
}

#[tokio::test]
async fn control_direct_head_accepts_representation_content_length() {
    let (io, mut srv) = mock::new();
    let remote = async move {
        srv.assert_client_handshake().await;
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
        idle_ms(10).await;
    };
    let local = async move {
        let (mut client, mut conn) = client::handshake(io).await.unwrap();
        let (response, _) = client
            .send_request(
                Request::head("https://example.com/").body(()).unwrap(),
                true,
            )
            .unwrap();
        let work = async move {
            let mut body = response.await.unwrap().into_body();
            let result = body.data().await;
            assert!(
                matches!(result, Some(Ok(ref data)) if data.is_empty()),
                "empty 304 body rejected: {result:?}"
            );
        };
        conn.drive(work).await;
    };
    join(remote, local).await;
}

#[tokio::test]
async fn control_unpadded_data_returns_consumed_capacity() {
    use futures::StreamExt;
    let (io, mut srv) = mock::new();
    let remote = async move {
        let settings = srv.assert_client_handshake().await;
        assert_eq!(settings.initial_window_size(), Some(16));
        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://example.com/")
                .eos(),
        )
        .await;
        srv.send_frame(frames::headers(1).response(200)).await;
        // Sixteen flow-controlled octets: pad length, one body octet, fourteen padding octets.
        let mut padded = vec![0, 0, 16, 0, 8, 0, 0, 0, 1, 14, b'x'];
        padded.extend([0; 14]);
        srv.send_frame(frames::data(1, "xxxxxxxxxxxxxxxx")).await;
        let result = tokio::time::timeout(Duration::from_millis(100), srv.next()).await;
        assert!(
            result.is_ok(),
            "peer exhausted its 16-byte window but no WINDOW_UPDATE arrived after body consumption"
        );
        let received = result.unwrap().unwrap().unwrap();
        assert!(
            matches!(received, h2::frame::Frame::WindowUpdate(_)),
            "expected WINDOW_UPDATE, got {received:?}"
        );
        srv.send_frame(frames::data(1, "y").eos()).await;
    };
    let local = async move {
        let (mut client, mut conn) = client::Builder::new()
            .initial_window_size(16)
            .handshake::<_, Bytes>(io)
            .await
            .unwrap();
        let (response, _) = client
            .send_request(Request::get("https://example.com/").body(()).unwrap(), true)
            .unwrap();
        let work = async move {
            let mut body = response.await.unwrap().into_body();
            let data = body.data().await.unwrap().unwrap();
            assert_eq!(data, "xxxxxxxxxxxxxxxx");
            body.flow_control().release_capacity(data.len()).unwrap();
            assert_eq!(body.data().await.unwrap().unwrap(), "y");
        };
        conn.drive(work).await;
    };
    join(remote, local).await;
}

#[tokio::test]
async fn control_large_reservations_below_four_gib() {
    let (io, _remote) = mock::new();
    let (mut client, _conn) = client::handshake(io).await.unwrap();
    let (_, mut body) = client
        .send_request(
            Request::post("https://example.com/").body(()).unwrap(),
            false,
        )
        .unwrap();
    for requested in [65535, 1usize << 31, (1usize << 32) - 1] {
        body.reserve_capacity(requested);
        assert_eq!(body.capacity(), 65535, "requested {requested}");
    }
}
