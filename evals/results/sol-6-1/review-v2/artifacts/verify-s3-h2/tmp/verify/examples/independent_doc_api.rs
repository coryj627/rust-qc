use h2::server::Connection;
use bytes::Bytes;
use tokio::io::{AsyncRead, AsyncWrite};
use std::task::Context;
#[allow(dead_code)]
fn drive<T: AsyncRead + AsyncWrite + Unpin>(conn: &mut Connection<T, Bytes>, cx: &mut Context<'_>) {
    let _ = conn.poll_closed(cx);
}
fn main() {}
