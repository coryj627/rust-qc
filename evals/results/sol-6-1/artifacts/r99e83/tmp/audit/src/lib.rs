use bytes::Bytes;
use futures::StreamExt;
use tokio::io::{AsyncWriteExt, AsyncReadExt};

fn frame(ty: u8, flags: u8, id: u32, payload: &[u8]) -> Vec<u8> {
    let mut out = vec![(payload.len() >> 16) as u8, (payload.len() >> 8) as u8, payload.len() as u8, ty, flags];
    out.extend(id.to_be_bytes());
    out.extend(payload);
    out
}
async fn codec(wire: Vec<u8>) -> h2::Codec<tokio::io::DuplexStream, Bytes> {
    let (mut tx, rx) = tokio::io::duplex(wire.len() + 1);
    tx.write_all(&wire).await.unwrap();
    h2::Codec::new(rx)
}
#[tokio::test]
async fn header_count_panic() {
    let mut fields = vec![0x82, 0x87, 0x84];
    fields.extend(vec![0x90; 32769]); // accept-encoding: gzip, deflate
    let mut wire = Vec::new();
    for (i, chunk) in fields.chunks(16384).enumerate() {
        wire.extend(frame(if i == 0 {1} else {9}, if (i+1)*16384 >= fields.len() {4} else {0}, 1, chunk));
    }
    let mut c = codec(wire).await;
    println!("{:?}", c.next().await);
}
#[tokio::test]
async fn malformed_fragment_forgotten() {
    // Duplicate :path followed by an incomplete literal.
    let mut wire = frame(1, 0, 1, &[0x82, 0x87, 0x84, 0x85, 0x40]);
    wire.extend(frame(9, 4, 1, &[1, b'x', 1, b'y']));
    let mut c = codec(wire).await;
    let decoded = c.next().await.unwrap().unwrap();
    if let h2::frame::Frame::Headers(h) = decoded {
        println!("accepted duplicate path: {:?}", h.into_parts());
    } else { panic!("wrong frame"); }
}
async fn server(wire: Vec<u8>) -> (h2::server::Connection<tokio::io::DuplexStream, Bytes>, tokio::io::DuplexStream) {
    let (mut peer, rx) = tokio::io::duplex(1024*1024);
    peer.write_all(b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n").await.unwrap();
    peer.write_all(&frame(4, 0, 0, &[])).await.unwrap();
    peer.write_all(&frame(4, 1, 0, &[])).await.unwrap();
    peer.write_all(&wire).await.unwrap();
    (h2::server::handshake(rx).await.unwrap(), peer)
}
#[tokio::test]
async fn missing_request_path() {
    let wire = frame(1, 5, 1, &[0x82, 0x87]);
    let (mut srv, _peer) = server(wire).await;
    println!("missing path accepted: {:?}", srv.accept().await);
}
#[tokio::test]
async fn padded_data_window() {
    let mut wire = frame(1, 4, 1, &[0x82, 0x87, 0x84]);
    let mut payload = vec![255];
    payload.extend(vec![0; 255]);
    for _ in 0..256 { wire.extend(frame(0, 8, 1, &payload)); }
    let (mut srv, mut peer) = server(wire).await;
    let (mut req, _send) = srv.accept().await.unwrap().unwrap();
    // Drive all frames without blocking.
    assert!(tokio::time::timeout(std::time::Duration::from_millis(10), srv.accept()).await.is_err());
    let mut count = 0;
    while let Ok(Some(Ok(data))) = tokio::time::timeout(std::time::Duration::from_millis(10), req.body_mut().data()).await {
        assert!(data.is_empty()); count += 1;
    }
    let mut reply = vec![0; 10000];
    let len = tokio::time::timeout(std::time::Duration::from_millis(10), peer.read(&mut reply)).await.unwrap().unwrap();
    println!("accepted {count} padded DATA frames = 65536 flow-controlled bytes; outbound {:?}", &reply[..len]);
    assert_eq!(count, 256);
}
