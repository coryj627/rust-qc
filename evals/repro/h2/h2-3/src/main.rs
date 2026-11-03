// h2-3 / PR #921 (fix 7c97e89).
// When the underlying transport's poll_write returns Ok(0) with bytes still
// queued, the flush loop in framed_write.rs retries forever instead of erroring.
// Because the loop never yields and never returns Poll::Pending, this is a CPU
// busy-loop (livelock), not a graceful stall: a single poll of the connection
// future never returns.
//
// We run the client connection on a dedicated thread with a transport that
// returns Ok(0) for every write once armed, and watch a "finished" flag from
// the main thread. At the baseline the connection future never completes, so
// the flag stays false past the timeout. The fix returns a WriteZero error
// promptly. Exit 0 iff the connection does NOT finish within the timeout
// (i.e. it is spinning) at this source tree.
use h2_support::prelude::*;
use std::io;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering::SeqCst};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::io::ReadBuf;

struct ZeroWriteIo<T> {
    inner: T,
    zero: Arc<AtomicBool>,
}

impl<T: AsyncRead + Unpin> AsyncRead for ZeroWriteIo<T> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}
impl<T: AsyncWrite + Unpin> AsyncWrite for ZeroWriteIo<T> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        if self.zero.load(SeqCst) {
            return Poll::Ready(Ok(0));
        }
        Pin::new(&mut self.inner).poll_write(cx, buf)
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

fn main() {
    let finished = Arc::new(AtomicBool::new(false));
    let f2 = finished.clone();

    let worker = std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        rt.block_on(async move {
            let (io, mut srv) = mock::new();
            let zero = Arc::new(AtomicBool::new(false));
            let io = ZeroWriteIo {
                inner: io,
                zero: zero.clone(),
            };
            let srv = async move {
                let _ = srv.assert_client_handshake().await;
                srv.recv_frame(
                    frames::headers(1)
                        .request("GET", "https://example.com/")
                        .eos(),
                )
                .await;
                srv.send_frame(frames::headers(1).response(200).eos()).await;
                idle_ms(2_000).await;
            };
            let client = async move {
                let (mut client, mut conn) = client::handshake(io).await.expect("handshake");
                let req = Request::builder()
                    .uri("https://example.com/")
                    .body(())
                    .unwrap();
                let (resp, _) = client.send_request(req, true).unwrap();
                let resp = conn.drive(resp).await.expect("response");
                assert_eq!(resp.status(), StatusCode::OK);
                // The transport's write side dies now.
                zero.store(true, SeqCst);
                // Queue another request and drive: fixed code errors WriteZero,
                // baseline spins forever inside a single poll.
                let req = Request::builder()
                    .uri("https://example.com/")
                    .body(())
                    .unwrap();
                let (_r, _) = client.send_request(req, true).unwrap();
                let res = conn.await;
                println!("connection returned: {:?}", res.err().map(|e| e.to_string()));
            };
            tokio::join!(srv, client);
        });
        f2.store(true, SeqCst);
    });

    // Watch for completion. The baseline livelocks and never sets the flag.
    let timeout = Duration::from_secs(4);
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if finished.load(SeqCst) {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let done = finished.load(SeqCst);
    // We deliberately leak the worker thread if it is still spinning.
    let _ = &worker;
    if !done {
        println!("DEMONSTRATED: flush busy-loops on Ok(0) write (connection never completed in {:?}, thread at 100% CPU)", timeout);
        std::process::exit(0);
    } else {
        println!("NOT demonstrated: connection completed (write-zero handled)");
        std::process::exit(1);
    }
}
