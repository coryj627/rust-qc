// h2-6 / Issue #958 / PR #959 (fix 3c5f61c).
// RFC 9113 section 8.3.2 requires every response to carry a :status pseudo-header.
// At the baseline, convert_poll_message sets the status only when present and has
// no else branch, so a response HEADERS with no :status is accepted and surfaced
// to the client with http::Response's default status of 200 OK instead of a
// PROTOCOL_ERROR stream reset.
//
// Exit 0 iff the client surfaces a 200 OK response for a status-less HEADERS
// frame (the bug).
use h2_support::prelude::*;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let (io, mut srv) = mock::new();

    let srv = async move {
        srv.assert_client_handshake().await;
        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://example.com/")
                .eos(),
        )
        .await;
        // Response HEADERS with END_STREAM but NO :status pseudo-header.
        srv.send_frame(frames::headers(1).eos()).await;
        idle_ms(100).await;
    };

    let client = async move {
        let (mut client, conn) = client::handshake(io).await.unwrap();
        tokio::spawn(async move {
            let _ = conn.await;
        });
        let req = Request::get("https://example.com/").body(()).unwrap();
        let (resp, _) = client.send_request(req, true).unwrap();
        resp.await
    };

    let (_, result) = tokio::join!(srv, client);
    match result {
        Ok(resp) if resp.status() == StatusCode::OK => {
            println!(
                "DEMONSTRATED: status-less response accepted as {} (default), RFC 9113 8.3.2 requires a stream error",
                resp.status()
            );
            std::process::exit(0);
        }
        Ok(resp) => {
            println!("NOT demonstrated: unexpected status {}", resp.status());
            std::process::exit(2);
        }
        Err(e) => {
            println!("NOT demonstrated: client rejected the response: {:?}", e);
            std::process::exit(1);
        }
    }
}
