use h2_support::prelude::*;
use std::{io, pin::Pin, sync::{Arc, atomic::{AtomicUsize, Ordering}}, task::{Context, Poll}};

struct ZeroWriter { calls: Arc<AtomicUsize> }
impl AsyncRead for ZeroWriter {
    fn poll_read(self: Pin<&mut Self>, _: &mut Context<'_>, _: &mut tokio::io::ReadBuf<'_>) -> Poll<io::Result<()>> { Poll::Pending }
}
impl AsyncWrite for ZeroWriter {
    fn poll_write(self: Pin<&mut Self>, _: &mut Context<'_>, data: &[u8]) -> Poll<io::Result<usize>> {
        match self.calls.fetch_add(1, Ordering::SeqCst) {
            0 => Poll::Ready(Ok(data.len())), // accept the connection preface
            1 => Poll::Ready(Ok(0)), // the transport cannot write any more data
            _ => Poll::Ready(Err(io::Error::new(io::ErrorKind::BrokenPipe, "bounded test sentinel: unexpected retry after zero write"))),
        }
    }
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> { Poll::Ready(Ok(())) }
    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> { Poll::Ready(Ok(())) }
}

#[tokio::test]
async fn zero_write_is_reported_without_retrying() {
    let calls = Arc::new(AtomicUsize::new(0));
    let (_client, connection) = client::handshake(ZeroWriter { calls: calls.clone() }).await.unwrap();
    let error = connection.await.unwrap_err();
    assert_eq!(error.get_io().unwrap().kind(), io::ErrorKind::WriteZero, "actual error={error:?}; write calls={}", calls.load(Ordering::SeqCst));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}
