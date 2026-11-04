use futures::StreamExt;
use h2_support::prelude::*;

fn request() -> Request<()> {
    Request::post("https://example.com/").body(()).unwrap()
}

async fn queued_upload(reserve: bool) {
    let (io, mut peer) = mock::new();
    let (ready_tx, ready_rx) = futures::channel::oneshot::channel();
    let (opened_tx, opened_rx) = futures::channel::oneshot::channel();
    let peer_task = async move {
        peer.assert_client_handshake_with_settings(frames::settings().max_concurrent_streams(1))
            .await;
        ready_tx.send(()).unwrap();
        peer.recv_frame(frames::headers(1).request("POST", "https://example.com/"))
            .await;
        opened_tx.send(()).unwrap();
        peer.recv_frame(frames::data(1, "a").eos()).await;
        peer.send_frame(frames::headers(1).response(200).eos())
            .await;
    };
    let client_task = async move {
        let (mut client, mut conn) = client::handshake(io).await.unwrap();
        conn.drive(ready_rx).await.unwrap();
        client = client.ready().await.unwrap();
        let (response1, mut body1) = client.send_request(request(), false).unwrap();
        conn.drive(opened_rx).await.unwrap();
        client = client.ready().await.unwrap();
        let (_response2, mut body2) = client.send_request(request(), false).unwrap();
        if reserve {
            body2.reserve_capacity(65_535);
        }
        body2.send_data(vec![b'b'; 65_535].into(), true).unwrap();
        body1.reserve_capacity(1);
        eprintln!(
            "active capacity={}, queued capacity={}",
            body1.capacity(),
            body2.capacity()
        );
        body1.send_data(Bytes::from_static(b"a"), true).unwrap();
        conn.drive(response1).await.unwrap();
    };
    tokio::time::timeout(Duration::from_secs(1), join(peer_task, client_task))
        .await
        .expect("an active request must be able to finish when another request is queued");
}

#[tokio::test]
async fn queued_upload_without_reservation() {
    queued_upload(false).await;
}

#[tokio::test]
async fn queued_reservation_must_not_block_active_request() {
    queued_upload(true).await;
}

async fn replenish_received_frame(padded: bool) {
    let (io, mut peer) = mock::new();
    let peer_task = async move {
        let settings = peer.assert_client_handshake().await;
        assert_eq!(settings.initial_window_size(), Some(5));
        peer.recv_frame(
            frames::headers(1)
                .request("POST", "https://example.com/")
                .eos(),
        )
        .await;
        peer.send_frame(frames::headers(1).response(200)).await;
        if padded {
            // Five flow-controlled octets: pad length, one data byte, three padding bytes.
            peer.send_bytes(&[0, 0, 5, 0, 8, 0, 0, 0, 1, 3, b'a', 0, 0, 0])
                .await;
        } else {
            peer.send_frame(frames::data(1, "aaaaa")).await;
        }
        let update = match peer.next().await.unwrap().unwrap() {
            h2::frame::Frame::WindowUpdate(update) => update,
            other => panic!("expected receive-window replenishment, got {:?}", other),
        };
        assert_eq!(update.stream_id(), 1);
        assert!(update.size_increment() > 0);
        peer.send_frame(frames::data(1, "b").eos()).await;
    };
    let client_task = async move {
        let (mut client, mut conn) = client::Builder::new()
            .initial_window_size(5)
            .handshake::<_, Bytes>(io)
            .await
            .unwrap();
        let (response, _) = client.send_request(request(), true).unwrap();
        let mut body = conn.drive(response).await.unwrap().into_body();
        let first = conn.drive(body.data()).await.unwrap().unwrap();
        assert_eq!(
            first.as_ref(),
            if padded {
                b"a".as_slice()
            } else {
                b"aaaaa".as_slice()
            }
        );
        body.flow_control().release_capacity(first.len()).unwrap();
        let second = conn.drive(body.data()).await.unwrap().unwrap();
        assert_eq!(second.as_ref(), b"b");
    };
    tokio::time::timeout(Duration::from_secs(1), join(peer_task, client_task))
        .await
        .expect("releasing all received data must let a conforming peer send its next frame");
}

#[tokio::test]
async fn unpadded_receive_window_replenishes() {
    replenish_received_frame(false).await;
}

#[tokio::test]
async fn padded_receive_window_replenishes() {
    replenish_received_frame(true).await;
}

async fn receive_not_modified(headers_eos: bool) {
    let (io, mut peer) = mock::new();
    let peer_task = async move {
        peer.assert_client_handshake().await;
        peer.recv_frame(
            frames::headers(1)
                .request("GET", "https://example.com/")
                .field("if-none-match", r#""version""#)
                .eos(),
        )
        .await;
        let headers = frames::headers(1)
            .response(304)
            .field("content-length", "123");
        if headers_eos {
            peer.send_frame(headers.eos()).await;
        } else {
            peer.send_frame(headers).await;
            peer.send_frame(frames::data(1, "").eos()).await;
        }
    };
    let client_task = async move {
        let (mut client, mut conn) = client::handshake(io).await.unwrap();
        let (response, _) = client
            .send_request(
                Request::get("https://example.com/")
                    .header("if-none-match", r#""version""#)
                    .body(())
                    .unwrap(),
                true,
            )
            .unwrap();
        let mut body = conn
            .drive(response)
            .await
            .expect("304 response headers")
            .into_body();
        let end = conn.drive(body.data()).await;
        assert!(
            end.is_none(),
            "304 must finish without a body error: {:?}",
            end
        );
    };
    join(peer_task, client_task).await;
}

#[tokio::test]
async fn not_modified_ends_with_headers() {
    receive_not_modified(true).await;
}

#[tokio::test]
async fn not_modified_ends_with_empty_data() {
    receive_not_modified(false).await;
}

async fn receive_head_response(pushed: bool) {
    let (io, mut peer) = mock::new();
    let peer_task = async move {
        peer.assert_client_handshake().await;
        peer.recv_frame(
            frames::headers(1)
                .request("HEAD", "https://example.com/")
                .eos(),
        )
        .await;
        if pushed {
            peer.send_frame(
                frames::push_promise(1, 2).request("HEAD", "https://example.com/other"),
            )
            .await;
            peer.send_frame(
                frames::headers(2)
                    .response(200)
                    .field("content-length", "123")
                    .eos(),
            )
            .await;
        }
        peer.send_frame(
            frames::headers(1)
                .response(200)
                .field("content-length", "123")
                .eos(),
        )
        .await;
    };
    let client_task = async move {
        let (mut client, mut conn) = client::handshake(io).await.unwrap();
        let req = Request::head("https://example.com/").body(()).unwrap();
        let (mut response, _) = client.send_request(req, true).unwrap();
        let mut promises = response.push_promises();
        if pushed {
            let promise = conn.drive(promises.push_promise()).await.unwrap().unwrap();
            let (req, pushed_response) = promise.into_parts();
            assert_eq!(req.method(), Method::HEAD);
            conn.drive(pushed_response)
                .await
                .expect("a HEAD response may advertise its GET representation length");
        }
        conn.drive(response).await.expect("regular HEAD response");
    };
    tokio::time::timeout(Duration::from_secs(1), join(peer_task, client_task))
        .await
        .unwrap();
}

#[tokio::test]
async fn regular_head_with_content_length() {
    receive_head_response(false).await;
}

#[tokio::test]
async fn pushed_head_with_content_length() {
    receive_head_response(true).await;
}

#[tokio::test]
async fn reservation_larger_than_u32_must_not_become_zero() {
    let (io, _peer) = mock::new();
    let (mut client, _conn) = client::handshake(io).await.unwrap();
    let (_response, mut body) = client.send_request(request(), false).unwrap();
    body.reserve_capacity(65_536);
    assert_eq!(body.capacity(), 65_535);
    body.reserve_capacity(1_usize << 32);
    assert_eq!(
        body.capacity(),
        65_535,
        "a 4 GiB intent should still reserve the currently available window"
    );
}

#[derive(Debug)]
struct ZeroWriter {
    calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}
impl AsyncRead for ZeroWriter {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        _buf: &mut tokio_io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::task::Poll::Pending
    }
}
impl AsyncWrite for ZeroWriter {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        _buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        // Bound this regression test: a second write reports a distinct error.
        // A normal permanently exhausted writer can keep returning Ok(0).
        if self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
            std::task::Poll::Ready(Ok(0))
        } else {
            std::task::Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "unexpected retry after zero-byte write",
            )))
        }
    }
    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::task::Poll::Ready(Ok(()))
    }
    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::task::Poll::Ready(Ok(()))
    }
}

#[tokio::test]
async fn zero_byte_write_must_return_write_zero() {
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let error = server::handshake(ZeroWriter {
        calls: calls.clone(),
    })
    .await
    .unwrap_err();
    eprintln!(
        "zero writer calls={}, error={error}",
        calls.load(std::sync::atomic::Ordering::SeqCst)
    );
    assert_eq!(
        error.get_io().unwrap().kind(),
        std::io::ErrorKind::WriteZero
    );
}
