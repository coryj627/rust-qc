use h2_support::prelude::*;
use futures::StreamExt;

async fn window_is_replenished(padded: bool) {
    let (io, mut peer) = mock::new();
    let peer_task = async move {
        peer.assert_client_handshake().await;
        peer.recv_frame(frames::headers(1).request("GET", "https://example.com/").eos()).await;
        peer.send_frame(frames::headers(1).response(200)).await;
        if padded {
            // Eight flow-controlled octets: padding length + one body byte + six padding bytes.
            peer.send_frame(frames::data(1, [6, b'x', 0, 0, 0, 0, 0, 0]).padded()).await;
        } else {
            peer.send_frame(frames::data(1, [b'x'; 8])).await;
        }
        let update = tokio::time::timeout(Duration::from_millis(100), peer.next()).await;
        assert!(update.is_ok(), "peer used its eight-octet stream window and needs WINDOW_UPDATE to continue; padded={padded}");
        match update.unwrap().unwrap().unwrap() {
            h2::frame::Frame::WindowUpdate(update) => assert_eq!(update.stream_id(), 1),
            other => panic!("expected stream WINDOW_UPDATE, actual={other:?}"),
        }
        idle_ms(250).await;
    };
    let client_task = async move {
        let (mut client, mut connection) = client::Builder::new().initial_window_size(8).handshake::<_, Bytes>(io).await.unwrap();
        let (response, _body) = client.send_request(Request::get("https://example.com/").body(()).unwrap(), true).unwrap();
        let mut body = connection.drive(response).await.unwrap().into_body();
        let chunk = connection.drive(body.data()).await.unwrap().unwrap();
        assert_eq!(chunk.len(), if padded { 1 } else { 8 });
        body.flow_control().release_capacity(chunk.len()).unwrap();
        connection.drive(tokio::time::sleep(Duration::from_millis(200))).await;
    };
    join(peer_task, client_task).await;
}

#[tokio::test]
async fn padded_data_releases_the_whole_consumed_window() { window_is_replenished(true).await; }

#[tokio::test]
async fn unpadded_data_control_releases_the_whole_consumed_window() { window_is_replenished(false).await; }
