use probe::*;
use std::pin::Pin;
use std::task::{Context, Poll};
struct W(usize);
impl tokio::io::AsyncRead for W { fn poll_read(self: Pin<&mut Self>, _: &mut Context<'_>, _: &mut tokio::io::ReadBuf<'_>) -> Poll<std::io::Result<()>> { Poll::Pending } }
impl tokio::io::AsyncWrite for W {
    fn poll_write(mut self: Pin<&mut Self>, _: &mut Context<'_>, b: &[u8]) -> Poll<std::io::Result<usize>> {
        if self.0 > 0 { let n = b.len().min(self.0); self.0 -= n; Poll::Ready(Ok(n)) } else { Poll::Ready(Ok(0)) }
    }
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> { Poll::Ready(Ok(())) }
    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> { Poll::Ready(Ok(())) }
}
#[test]
fn ok0() {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(async move {
            let (_c, conn) = client::handshake(W(24)).await.unwrap();
            let r = timeout(Duration::from_millis(300), conn).await;
            tx.send(format!("returned {:?}", r.map(|x| x.is_ok()))).unwrap();
        });
    });
    match rx.recv_timeout(Duration::from_secs(4)) {
        Ok(s) => println!("ok0: {s}"),
        Err(_) => { println!("ok0: SPINNING: timeout(300ms) did not fire within 4s"); std::process::exit(0); }
    }
}
