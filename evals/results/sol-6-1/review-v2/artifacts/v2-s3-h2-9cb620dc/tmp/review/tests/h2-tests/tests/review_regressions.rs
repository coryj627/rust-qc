use futures::StreamExt;
use h2_support::prelude::*;
use std::io;
use std::pin::Pin;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::task::{Context, Poll};

#[tokio::test]
async fn reserve_capacity_four_gib() {
    let (io, _srv) = mock::new();
    let (mut client, _conn) = client::handshake(io).await.unwrap();
    let (_, mut body) = client
        .send_request(
            Request::post("https://example.com/").body(()).unwrap(),
            false,
        )
        .unwrap();
    body.reserve_capacity(65535);
    assert_eq!(body.capacity(), 65535, "control reservation");
    body.reserve_capacity(1usize << 32);
    assert!(
        body.capacity() > 0,
        "a 4 GiB reservation must retain available capacity; got {}",
        body.capacity()
    );
}

async fn not_modified_response(end_in_headers: bool) {
    let (io, mut srv) = mock::new();
    let peer = async move {
        srv.assert_client_handshake().await;
        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://example.com/")
                .eos(),
        )
        .await;
        let headers = frames::headers(1)
            .response(304)
            .field("content-length", 100);
        if end_in_headers {
            srv.send_frame(headers.eos()).await;
        } else {
            srv.send_frame(headers).await;
            srv.send_frame(frames::data(1, "").eos()).await;
        }
    };
    let app = async move {
        let (mut client, mut conn) = client::handshake(io).await.unwrap();
        let (response, _) = client
            .send_request(Request::get("https://example.com/").body(()).unwrap(), true)
            .unwrap();
        conn.drive(async move {
            let response = response.await.expect("valid 304 response");
            assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
            let mut body = response.into_body();
            while let Some(data) = body.data().await {
                assert!(data.expect("304 may end with empty DATA").is_empty());
            }
        })
        .await;
    };
    join(peer, app).await;
}

#[tokio::test]
async fn not_modified_ends_in_headers_control() {
    not_modified_response(true).await;
}

#[tokio::test]
async fn not_modified_ends_in_empty_data() {
    not_modified_response(false).await;
}

async fn data_restores_progress(padded: bool) {
    let (io, mut srv) = mock::new();
    let peer = async move {
        let settings = srv.assert_client_handshake().await;
        assert_eq!(settings.initial_window_size(), Some(16));
        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://example.com/")
                .eos(),
        )
        .await;
        srv.send_frame(frames::headers(1).response(200)).await;
        let mut wire_body = vec![14, b'x'];
        wire_body.extend_from_slice(&[0; 14]);
        if padded {
            srv.send_frame(frames::data(1, wire_body).padded()).await;
        } else {
            srv.send_frame(frames::data(1, "xxxxxxxxxxxxxxxx")).await;
        }
        let update = tokio::time::timeout(Duration::from_secs(1), srv.next()).await;
        assert!(update.is_ok(), "peer used its full 16-byte stream window, app consumed data, but received no WINDOW_UPDATE: {:?}", update);
        match update.unwrap().unwrap().unwrap() {
            frame::Frame::WindowUpdate(update) => {
                assert_eq!(update.stream_id(), 1u32);
                assert!(update.size_increment() > 0);
            }
            other => panic!("expected stream WINDOW_UPDATE, got {:?}", other),
        }
        srv.send_frame(frames::data(1, "").eos()).await;
    };
    let app = async move {
        let (mut client, mut conn) = client::Builder::new()
            .initial_window_size(16)
            .handshake::<_, Bytes>(io)
            .await
            .unwrap();
        let (response, _) = client
            .send_request(Request::get("https://example.com/").body(()).unwrap(), true)
            .unwrap();
        conn.drive(async move {
            let mut body = response.await.unwrap().into_body();
            let data = body.data().await.unwrap().unwrap();
            assert_eq!(data.len(), if padded { 1 } else { 16 });
            body.flow_control().release_capacity(data.len()).unwrap();
            while let Some(data) = body.data().await {
                data.unwrap();
            }
        })
        .await;
    };
    join(peer, app).await;
}

struct ZeroThenError(Arc<AtomicUsize>);
impl AsyncRead for ZeroThenError {
    fn poll_read(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        _: &mut tokio_io::ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Poll::Pending
    }
}
impl AsyncWrite for ZeroThenError {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        assert!(!buf.is_empty());
        if self.0.fetch_add(1, Ordering::SeqCst) == 0 {
            Poll::Ready(Ok(0))
        } else {
            Poll::Ready(Err(io::Error::new(
                io::ErrorKind::Other,
                "second write sentinel",
            )))
        }
    }
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[tokio::test]
async fn zero_write_returns_write_zero() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut codec = h2::Codec::<_, Bytes>::new(ZeroThenError(calls.clone()));
    codec.buffer(frame::Settings::default().into()).unwrap();
    let error = poll_fn(|cx| codec.flush(cx)).await.unwrap_err();
    assert_eq!(
        (error.kind(), calls.load(Ordering::SeqCst)),
        (io::ErrorKind::WriteZero, 1),
        "error={error}"
    );
}

#[tokio::test]
async fn padded_data_restores_progress() {
    data_restores_progress(true).await;
}

#[tokio::test]
async fn unpadded_data_restores_progress_control() {
    data_restores_progress(false).await;
}

#[tokio::test]
async fn zero_write_server_handshake() {
    let calls = Arc::new(AtomicUsize::new(0));
    let error = server::handshake(ZeroThenError(calls.clone()))
        .await
        .err()
        .expect("write error");
    assert_eq!(
        (error.get_io().unwrap().kind(), calls.load(Ordering::SeqCst)),
        (io::ErrorKind::WriteZero, 1),
        "error={error}"
    );
}

async fn failed_push_does_not_keep_stream(method: &str, connection_header: bool) {
    let (io, mut peer) = mock::new();
    let client = async move {
        peer.assert_server_handshake().await;
        peer.send_frame(
            frames::headers(1)
                .request("GET", "https://example.com/")
                .eos(),
        )
        .await;
        peer.recv_frame(frames::headers(1).response(200).eos())
            .await;
    };
    let app = async move {
        let mut conn = server::handshake(io).await.unwrap();
        let (req, mut responder) = conn.accept().await.unwrap().unwrap();
        let before = conn.num_wired_streams();
        let mut request = Request::builder()
            .method(method)
            .uri("https://example.com/pushed");
        if connection_header {
            request = request.header("connection", "close");
        }
        let error = responder
            .push_request(request.body(()).unwrap())
            .unwrap_err();
        let after = conn.num_wired_streams();
        assert_eq!(
            after, before,
            "failed push retained an extra stream: {error}"
        );
        responder.send_response(Response::new(()), true).unwrap();
        drop((req, responder));
        assert!(conn.accept().await.is_none());
    };
    join(client, app).await;
}

#[tokio::test]
async fn rejected_post_push_is_removed() {
    failed_push_does_not_keep_stream("POST", false).await;
}

#[tokio::test]
async fn rejected_connection_header_push_is_removed_control() {
    failed_push_does_not_keep_stream("GET", true).await;
}

#[tokio::test]
async fn pushed_response_allows_early_hints() {
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
        srv.send_frame(frames::push_promise(1, 2).request("GET", "https://example.com/style.css"))
            .await;
        srv.send_frame(frames::headers(2).response(103)).await;
        srv.send_frame(frames::headers(2).response(200).eos()).await;
        srv.send_frame(frames::data(1, "").eos()).await;
    };
    let app = async move {
        let (mut client, mut conn) = client::handshake(io).await.unwrap();
        let (mut response, _) = client
            .send_request(Request::get("https://example.com/").body(()).unwrap(), true)
            .unwrap();
        let mut pushes = response.push_promises();
        conn.drive(async move {
            let pushed = pushes
                .next()
                .await
                .unwrap()
                .unwrap()
                .into_parts()
                .1
                .await
                .unwrap();
            assert_eq!(pushed.status(), StatusCode::OK);
            let _response = response.await.unwrap();
        })
        .await;
    };
    join(peer, app).await;
}

#[tokio::test]
async fn rejected_push_persists_into_next_request() {
    let (io, mut peer) = mock::new();
    let client = async move {
        peer.assert_server_handshake().await;
        for id in [1, 3] {
            peer.send_frame(
                frames::headers(id)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;
            peer.recv_frame(frames::headers(id).response(200).eos())
                .await;
        }
    };
    let app = async move {
        let mut conn = server::handshake(io).await.unwrap();
        let (req, mut responder) = conn.accept().await.unwrap().unwrap();
        let error = responder
            .push_request(
                Request::post("https://example.com/pushed")
                    .body(())
                    .unwrap(),
            )
            .unwrap_err();
        assert_eq!(error.to_string(), "user error: malformed headers");
        responder.send_response(Response::new(()), true).unwrap();
        drop((req, responder));
        let (req, mut responder) = conn.accept().await.unwrap().unwrap();
        assert_eq!(
            conn.num_wired_streams(),
            1,
            "only the second request should remain stored"
        );
        responder.send_response(Response::new(()), true).unwrap();
        drop((req, responder));
        assert!(conn.accept().await.is_none());
    };
    join(client, app).await;
}
