use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

#[derive(Debug)]
struct FailedRead;

impl AsyncRead for FailedRead {
    fn poll_read(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        _: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Poll::Ready(Err(io::Error::from_raw_os_error(5)))
    }
}

impl AsyncWrite for FailedRead {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        Poll::Ready(Ok(bytes.len()))
    }
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[tokio::test]
async fn connection_io_error_preserves_os_code() {
    let (_client, conn) = h2::client::handshake(FailedRead).await.unwrap();
    let error = conn.await.unwrap_err();
    let io = error.get_io().expect("I/O errors remain I/O errors");
    println!(
        "returned error={io:?}, raw_os_error={:?}",
        io.raw_os_error()
    );
    assert_eq!(
        io.raw_os_error(),
        Some(5),
        "get_io must preserve the underlying OS error code"
    );
}
