use h2_support::prelude::*;

async fn available_after_reservation(requested: usize) -> usize {
    let (io, _peer) = mock::new();
    let (mut client, _connection) = client::handshake(io).await.unwrap();
    let request = Request::builder().uri("https://example.com/upload").body(()).unwrap();
    let (_response, mut body) = client.send_request(request, false).unwrap();
    body.reserve_capacity(requested);
    body.capacity()
}

#[tokio::test]
async fn ordinary_reservation_receives_capacity() {
    assert!(available_after_reservation(65535).await > 0);
}

#[tokio::test]
async fn four_gib_reservation_receives_capacity() {
    assert!(available_after_reservation(1usize << 32).await > 0);
}

#[tokio::test]
async fn largest_u32_reservation_receives_capacity() {
    assert!(available_after_reservation(u32::MAX as usize).await > 0);
}
