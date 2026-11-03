// h2-7 / Issue #885 / PR #886 (fix f5f1594).
// On receiving GOAWAY, recv_go_away closes every stream with id > last_stream_id
// regardless of which endpoint opened it. RFC 9113 section 6.8 says GOAWAY's
// last_stream_id refers only to streams the SENDER initiated. When a client sends
// GOAWAY(last_stream_id=0) to a server, the baseline wrongly closes the
// client-initiated stream 1, killing an in-progress server response body.
//
// System under test: the server. The mock plays the client. Exit 0 iff the
// server fails to deliver the response body for stream 1 after the client's
// GOAWAY(0) (the bug): we wait for the body DATA frame and it never arrives.
use h2_support::prelude::*;
use h2_support::prelude::futures::StreamExt;
use std::time::Duration;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let (io, mut client) = mock::new();

    let client_side = async move {
        let _ = client.assert_server_handshake().await;
        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;
        // Server response headers, no END_STREAM (body to follow).
        client.recv_frame(frames::headers(1).response(200)).await;
        // Client announces shutdown with last_stream_id = 0.
        client.send_frame(frames::go_away(0)).await;
        // The server should still deliver the body DATA on stream 1. Inspect the
        // next frames: a correct server sends Data{stream 1}; the baseline kills
        // stream 1 and sends GOAWAY instead.
        let mut got_body = false;
        for _ in 0..3 {
            match tokio::time::timeout(Duration::from_millis(1000), client.next()).await {
                Ok(Some(Ok(frame))) => {
                    let desc = format!("{:?}", frame);
                    println!("server emitted: {}", desc);
                    if desc.starts_with("Data") && desc.contains("StreamId(1)") {
                        got_body = true;
                        break;
                    }
                }
                _ => break,
            }
        }
        got_body
    };

    let server_side = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (req, mut stream) = srv.next().await.unwrap().unwrap();
        assert_eq!(req.method(), &http::Method::GET);
        let rsp = http::Response::builder().status(200).body(()).unwrap();
        let mut tx = stream.send_response(rsp, false).unwrap();
        let send_body = async {
            // Flush headers; client then sends GOAWAY(0).
            tokio::task::yield_now().await;
            // Let the connection process the GOAWAY (baseline kills stream 1 here).
            tokio::task::yield_now().await;
            let _ = tx.send_data("the response body".into(), true);
        };
        let mut srv = Box::pin(async move {
            let _ = srv.next().await;
        });
        srv.drive(send_body).await;
        // Give the connection time to flush the body if it still can.
        let _ = tokio::time::timeout(Duration::from_millis(1000), srv).await;
    };

    let (delivered, ()) = tokio::join!(client_side, server_side);
    if !delivered {
        println!("DEMONSTRATED: client GOAWAY(0) killed server response on client-initiated stream 1 (body never delivered)");
        std::process::exit(0);
    } else {
        println!("NOT demonstrated: server delivered the response body despite client GOAWAY(0)");
        std::process::exit(1);
    }
}
