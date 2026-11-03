// h2-8 / PR #895 (fix d9689ea).
// After h2 sends GOAWAY, DATA arriving on a stream with id above the GOAWAY
// last_stream_id is correctly ignored, but at the baseline the early return in
// streams.rs does NOT call ignore_data, so the connection-level flow-control
// window is never replenished for those bytes. The capacity leaks, and streams
// below last_stream_id can stall forever. The fix adds ignore_data(sz) before
// the return, so the server emits WINDOW_UPDATE(stream 0) for the ignored bytes.
//
// System under test: the server. Exit 0 iff, after the server has sent its final
// GOAWAY(last_stream_id=3) and the client sends two 16 KiB DATA frames on stream
// 5 (above 3), the server does NOT emit a connection-level WINDOW_UPDATE (the
// leak). The fix emits WINDOW_UPDATE(0, 32768).
use h2_support::prelude::*;
use h2_support::prelude::futures::StreamExt;
use std::sync::atomic::{AtomicBool, Ordering::SeqCst};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let (io, mut client) = mock::new();
    let saw_conn_window_update = Arc::new(AtomicBool::new(false));
    let flag = saw_conn_window_update.clone();

    let data = vec![0u8; 16_384];

    let client_side = async move {
        let _ = client.assert_server_handshake().await;
        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;
        // Graceful shutdown: GOAWAY(MAX) then PING.
        client.recv_frame(frames::go_away(2147483647)).await;
        client.recv_frame(frames::ping(frame::Ping::SHUTDOWN)).await;
        client
            .recv_frame(frames::headers(1).response(200).eos())
            .await;
        // Stream 3 arrives before the client acknowledges the shutdown.
        client
            .send_frame(frames::headers(3).request("POST", "https://example.com/"))
            .await;
        client
            .send_frame(frames::ping(frame::Ping::SHUTDOWN).pong())
            .await;
        // Final GOAWAY(3): streams 1 and 3 accepted, above-3 rejected.
        client.recv_frame(frames::go_away(3)).await;
        // Stream 5 is above last_stream_id = 3. Its DATA is ignored, but the
        // connection window must still be returned.
        client
            .send_frame(frames::headers(5).request("POST", "https://example.com/"))
            .await;
        client.send_frame(frames::data(5, &data[..])).await;
        client.send_frame(frames::data(5, &data[..])).await;

        // Look for a connection-level WINDOW_UPDATE replenishing the ignored
        // 32768 bytes. The fix sends it; the baseline leaks (none appears).
        for _ in 0..6 {
            match tokio::time::timeout(Duration::from_millis(600), client.next()).await {
                Ok(Some(Ok(frame))) => {
                    let desc = format!("{:?}", frame);
                    println!("server emitted: {}", desc);
                    if desc.starts_with("WindowUpdate") && desc.contains("StreamId(0)") {
                        flag.store(true, SeqCst);
                        break;
                    }
                }
                _ => break,
            }
        }
    };

    let server_side = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (_req, mut stream) = srv.next().await.unwrap().unwrap();
        srv.graceful_shutdown();
        let rsp = http::Response::builder().status(200).body(()).unwrap();
        stream.send_response(rsp, true).unwrap();

        let next = srv.next().await;
        if let Some(Ok((req, mut stream))) = next {
            let body = req.into_parts().1;
            let drive_body = async move {
                let _ = util::concat(body).await;
                let rsp = http::Response::builder().status(200).body(()).unwrap();
                let _ = stream.send_response(rsp, true);
            };
            let mut srv = Box::pin(async move {
                let _ = srv.next().await;
            });
            let _ = tokio::time::timeout(Duration::from_millis(2500), srv.drive(drive_body)).await;
        }
    };

    let _ = tokio::time::timeout(
        Duration::from_secs(6),
        async { tokio::join!(client_side, server_side) },
    )
    .await;

    if !saw_conn_window_update.load(SeqCst) {
        println!("DEMONSTRATED: no connection WINDOW_UPDATE for 32768 bytes of ignored DATA above last_stream_id (flow-control capacity leaked)");
        std::process::exit(0);
    } else {
        println!("NOT demonstrated: server returned connection flow-control capacity");
        std::process::exit(1);
    }
}
