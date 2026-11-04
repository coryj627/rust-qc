use bytes::Bytes;
use http::Request;

#[tokio::test]
async fn reservation_boundaries() {
    let mut failed = Vec::new();
    for size in [
        1024usize,
        (1usize << 31) - 1,
        1usize << 31,
        (1usize << 32) - 1,
        1usize << 32,
        (1usize << 32) + 1,
    ] {
        let (io, _peer) = tokio::io::duplex(4096);
        let (mut client, _conn) = h2::client::handshake(io).await.unwrap();
        let (_, mut body): (_, h2::SendStream<Bytes>) = client
            .send_request(
                Request::post("https://example.com/upload")
                    .body(())
                    .unwrap(),
                false,
            )
            .unwrap();
        body.reserve_capacity(size);
        println!("reservation {size}: capacity {}", body.capacity());
        if body.capacity() == 0 {
            failed.push(size);
        }
    }
    assert!(
        failed.is_empty(),
        "nonzero reservations that received no capacity: {failed:?}"
    );
}
