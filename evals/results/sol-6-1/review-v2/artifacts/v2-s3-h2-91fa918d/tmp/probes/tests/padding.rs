use futures::StreamExt;
use h2_support::prelude::*;

#[tokio::test]
async fn padding_only_frame_returns_window_capacity() {
    let (io, mut srv) = mock::new();
    let peer = async move {
        let settings = srv.assert_client_handshake().await;
        assert_eq!(settings.initial_window_size(), Some(256));
        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://example.com/")
                .eos(),
        )
        .await;
        srv.send_frame(frames::headers(1).response(200)).await;
        // Legal DATA frame: one pad-length octet and 255 padding octets.
        let mut payload = vec![0u8; 256];
        payload[0] = 255;
        srv.send_frame(frames::data(1, payload).padded()).await;
        let frame = tokio::time::timeout(Duration::from_millis(200), srv.next())
            .await
            .expect("consumed padding must return stream capacity so the peer can send body data")
            .expect("connection remains open")
            .unwrap();
        match frame {
            h2::frame::Frame::WindowUpdate(update) => {
                assert_eq!(update.stream_id(), 1);
                assert_eq!(update.size_increment(), 256);
            }
            other => panic!("expected window update, got {other:?}"),
        }
    };
    let consumer = async move {
        let (mut client, conn) = client::Builder::new()
            .initial_window_size(256)
            .handshake::<_, Bytes>(io)
            .await
            .unwrap();
        let driver = tokio::spawn(conn);
        let (response, _) = client
            .send_request(Request::get("https://example.com/").body(()).unwrap(), true)
            .unwrap();
        let mut body = response.await.unwrap().into_body();
        let data = body.data().await.unwrap().unwrap();
        assert!(data.is_empty());
        body.flow_control().release_capacity(data.len()).unwrap();
        println!(
            "after padding: available={}, used={}",
            body.flow_control().available_capacity(),
            body.flow_control().used_capacity()
        );
        // Retain the receive stream while the peer waits for capacity.
        tokio::time::sleep(Duration::from_millis(400)).await;
        driver.abort();
    };
    futures::future::join(peer, consumer).await;
}
