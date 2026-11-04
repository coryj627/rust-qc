use futures::{StreamExt, TryStreamExt};
use h2_support::prelude::*;

#[tokio::test]
async fn recv_head_push_with_content_length() {

    let (io, mut srv) = mock::new();
    let mock = async move {
        let settings = srv.assert_client_handshake().await;
        assert_default_settings!(settings);
        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://http2.akamai.com/")
                .eos(),
        )
        .await;
        srv.send_frame(frames::headers(1).response(404)).await;
        srv.send_frame(
            frames::push_promise(1, 2).request("HEAD", "https://http2.akamai.com/style.css"),
        )
        .await;
        srv.send_frame(frames::data(1, "").eos()).await;
        srv.send_frame(frames::headers(2).response(200).field("content-length", 100).eos()).await;
    };
    let h2 = async move {
        let (mut client, mut h2) = client::handshake(io).await.unwrap();
        let request = Request::builder()
            .method(Method::GET)
            .uri("https://http2.akamai.com/")
            .body(())
            .unwrap();
        let (mut resp, _) = client.send_request(request, true).unwrap();
        let pushed = resp.push_promises();
        let check_resp_status = async move {
            let resp = resp.await.unwrap();
            assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        };
        let check_pushed_response = async move {
            let p = pushed.and_then(|headers| async move {
                let (request, response) = headers.into_parts();
                assert_eq!(request.into_parts().0.method, Method::HEAD);
                let resp = response.await.unwrap();
                assert_eq!(resp.status(), StatusCode::OK);
                let b = util::concat(resp.into_body()).await.unwrap();
                assert_eq!(b, "");
                Ok(())
            });
            let ps: Vec<_> = p.collect().await;
            assert_eq!(1, ps.len())
        };

        h2.drive(join(check_resp_status, check_pushed_response))
            .await;
    };

    join(mock, h2).await;
}

#[tokio::test]
async fn recv_head_push_without_content_length() {

    let (io, mut srv) = mock::new();
    let mock = async move {
        let settings = srv.assert_client_handshake().await;
        assert_default_settings!(settings);
        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://http2.akamai.com/")
                .eos(),
        )
        .await;
        srv.send_frame(frames::headers(1).response(404)).await;
        srv.send_frame(
            frames::push_promise(1, 2).request("HEAD", "https://http2.akamai.com/style.css"),
        )
        .await;
        srv.send_frame(frames::data(1, "").eos()).await;
        srv.send_frame(frames::headers(2).response(200).eos()).await;
    };
    let h2 = async move {
        let (mut client, mut h2) = client::handshake(io).await.unwrap();
        let request = Request::builder()
            .method(Method::GET)
            .uri("https://http2.akamai.com/")
            .body(())
            .unwrap();
        let (mut resp, _) = client.send_request(request, true).unwrap();
        let pushed = resp.push_promises();
        let check_resp_status = async move {
            let resp = resp.await.unwrap();
            assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        };
        let check_pushed_response = async move {
            let p = pushed.and_then(|headers| async move {
                let (request, response) = headers.into_parts();
                assert_eq!(request.into_parts().0.method, Method::HEAD);
                let resp = response.await.unwrap();
                assert_eq!(resp.status(), StatusCode::OK);
                let b = util::concat(resp.into_body()).await.unwrap();
                assert_eq!(b, "");
                Ok(())
            });
            let ps: Vec<_> = p.collect().await;
            assert_eq!(1, ps.len())
        };

        h2.drive(join(check_resp_status, check_pushed_response))
            .await;
    };

    join(mock, h2).await;
}

