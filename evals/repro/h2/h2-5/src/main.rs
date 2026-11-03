// h2-5 / PR #967 (fix d96d347).
// RFC 9113 inherits RFC 9110 message framing: a message with two content-length
// header fields whose values differ is malformed and MUST be rejected. At the
// baseline, recv_headers reads only the first content-length via HeaderMap::get,
// so a response carrying content-length: 2 and content-length: 3 is accepted and
// surfaced to the client instead of a PROTOCOL_ERROR stream reset.
//
// Exit 0 iff the client accepts the conflicting response (the bug), i.e. the
// response resolves Ok rather than failing with a PROTOCOL_ERROR reset.
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
        // Two differing content-length values on the response.
        let mut fields = HeaderMap::new();
        fields.append("content-length", "2".parse().unwrap());
        fields.append("content-length", "3".parse().unwrap());
        srv.send_frame(frames::headers(1).response(200).fields(fields))
            .await;
        srv.send_frame(frames::data(1, "xx").eos()).await;
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
        Ok(resp) => {
            println!(
                "DEMONSTRATED: conflicting content-length response accepted (status {}); RFC 9110 requires rejection as malformed",
                resp.status()
            );
            std::process::exit(0);
        }
        Err(e) if e.reason() == Some(Reason::PROTOCOL_ERROR) => {
            println!("NOT demonstrated: client rejected with PROTOCOL_ERROR (correct)");
            std::process::exit(1);
        }
        Err(e) => {
            println!("NOT demonstrated: client failed for another reason: {:?}", e);
            std::process::exit(2);
        }
    }
}
