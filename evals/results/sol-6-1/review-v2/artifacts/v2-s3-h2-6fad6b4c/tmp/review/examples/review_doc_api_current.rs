use bytes::Bytes;
use h2::server::Connection;
use std::task::Context;
use tokio::io::{AsyncRead, AsyncWrite};

#[allow(dead_code)]
fn drive<T: AsyncRead + AsyncWrite + Unpin>(conn: &mut Connection<T, Bytes>, cx: &mut Context<'_>) {
    // This is the connection-driving method named by the server module guide.
    let _ = conn.poll_closed(cx);
}

fn main() {}
