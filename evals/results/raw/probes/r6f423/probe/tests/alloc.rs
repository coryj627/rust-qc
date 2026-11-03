#![allow(dead_code, unused)]
use h2_support::prelude::*;
use futures::StreamExt;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicIsize, Ordering::SeqCst};
use std::time::Duration;

struct C;
static LIVE: AtomicIsize = AtomicIsize::new(0);
unsafe impl GlobalAlloc for C {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 { LIVE.fetch_add(l.size() as isize, SeqCst); System.alloc(l) }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) { LIVE.fetch_sub(l.size() as isize, SeqCst); System.dealloc(p, l) }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 { LIVE.fetch_add(n as isize - l.size() as isize, SeqCst); System.realloc(p, l, n) }
}
#[global_allocator] static A: C = C;

fn get() -> Request<()> { Request::builder().method(Method::GET).uri("https://a/").body(()).unwrap() }

async fn run_resp(poll_it: bool) {
    let (io, mut srv) = mock::new();
    const N: usize = 2000;
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<u32>();
    let mock = async move {
        srv.assert_client_handshake().await;
        for i in 0..N as u32 {
            let id = 1 + 2 * i;
            let _ = srv.next().await; // HEADERS
            let mut f = frames::headers(id).response(200);
            // 4KB, never indexed on the wire by the mock encoder? value unique so memory is per response
            let v = String::from_utf8(vec![b'a' + (i % 26) as u8; 4096]).unwrap();
            f = f.field("x-big", v.as_str()).eos();
            srv.send_frame(f).await;
            tx.send(id).unwrap();
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    };
    let cl = async move {
        let (mut client, mut h2) = client::handshake(io).await.unwrap();
        let mut mid = 0; 
        for i in 0..N {
            let (resp, _) = client.send_request(get(), true).unwrap();
            let id = h2.drive(rx.recv()).await.unwrap();
            // make sure the response frame was read by the connection
            h2.drive(tokio::time::sleep(Duration::from_micros(200))).await;
            if poll_it { let r = h2.drive(resp).await.unwrap(); drop(r); } else { drop(resp); }
            h2.drive(tokio::time::sleep(Duration::from_micros(50))).await;
            if i == N / 4 { mid = LIVE.load(SeqCst); }
        }
        let end = LIVE.load(SeqCst);
        println!("poll_it={} live bytes after {} = {}, after {} = {}, growth per request = {}, wired = {}", poll_it, N/4, mid, N, end, (end-mid)/(N as isize*3/4), client.num_wired_streams());
        std::mem::forget((client, h2));
    };
    join(mock, cl).await;
}
#[tokio::test(flavor = "current_thread")] async fn a1_dropped_unpolled_response() { run_resp(false).await; }
#[tokio::test(flavor = "current_thread")] async fn a2_control_polled_response() { run_resp(true).await; }

// Encoder table honours whatever table size the peer asks for
async fn run_enc(size: u32) {
    let (io, mut c) = mock::new();
    const N: u32 = 3000;
    let peer = async move {
        c.assert_server_handshake_with_settings(frames::settings().header_table_size(size)).await;
        for i in 0..N {
            let id = 1 + 2 * i;
            let mut b = vec![0x82, 0x87, 0x84, 0x00, 10]; b.extend_from_slice(b":authority"); b.extend_from_slice(&[1, b'a']);
            let l = b.len();
            let mut f = vec![(l >> 16) as u8, (l >> 8) as u8, l as u8, 1, 5];
            f.extend_from_slice(&id.to_be_bytes()); f.extend_from_slice(&b);
            c.send_bytes(&f).await;
            let _ = c.next().await;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    };
    let s = async move {
        let mut srv = server::handshake(io).await.unwrap();
        let before = LIVE.load(SeqCst);
        let mut i = 0u32;
        let mut mid = 0;
        while let Some(Ok((_r, mut resp))) = srv.next().await {
            let v = format!("{:01000}", i);
            let rs = Response::builder().status(200).header("x-id", v).body(()).unwrap();
            resp.send_response(rs, true).unwrap();
            i += 1;
            if i == N / 4 { mid = LIVE.load(SeqCst); }
            if i == N { break; }
        }
        tokio::select!{ _ = srv.next() => {}, _ = tokio::time::sleep(Duration::from_millis(20)) => {} };
        let end = LIVE.load(SeqCst);
        println!("peer table size {}: live bytes growth over last {} responses = {} ({} B/response)", size, N*3/4, end - mid, (end-mid)/((N*3/4) as isize));
        std::mem::forget(srv);
    };
    join(peer, s).await;
}
#[tokio::test(flavor = "current_thread")] async fn e1_table_huge() { run_enc(u32::MAX).await; }
#[tokio::test(flavor = "current_thread")] async fn e2_table_default() { run_enc(4096).await; }

// Empty DATA frames consume no flow control but are queued per frame
async fn run_empty(pad: bool) {
    let (io, mut c) = mock::new();
    const N: usize = 200_000;
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<u8>();
    let peer = async move {
        c.assert_server_handshake().await;
        let mut b = vec![0x82, 0x87, 0x84, 0x00, 10]; b.extend_from_slice(b":authority"); b.extend_from_slice(&[1, b'a']);
        let l = b.len();
        let mut f = vec![(l >> 16) as u8, (l >> 8) as u8, l as u8, 1, 4, 0,0,0,1];
        f.extend_from_slice(&b);
        c.send_bytes(&f).await;
        tokio::time::sleep(Duration::from_millis(20)).await;
        let frame: Vec<u8> = if pad { vec![0,0,1, 0, 8, 0,0,0,1, 0] } else { vec![0,0,0, 0, 0, 0,0,0,1] };
        let mut chunk = vec![]; for _ in 0..1000 { chunk.extend_from_slice(&frame); }
        for _ in 0..(N/1000) { c.send_bytes(&chunk).await; tokio::task::yield_now().await; }
        tokio::time::sleep(Duration::from_millis(200)).await;
        tx.send(1).unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
    };
    let s = async move {
        let mut srv = server::handshake(io).await.unwrap();
        let (req, _resp) = srv.next().await.unwrap().unwrap();
        let before = LIVE.load(SeqCst);
        tokio::select!{ _ = srv.next() => {}, _ = rx.recv() => {} };
        let end = LIVE.load(SeqCst);
        println!("{} DATA frames with no payload (pad={}): server live bytes grew by {} ({} B/frame); available={}", N, pad, end-before, (end-before)/N as isize, 0);
        std::mem::forget((srv, req, _resp));
    };
    join(peer, s).await;
}
#[tokio::test(flavor = "current_thread")] async fn f1_empty_data_flood() { run_empty(false).await; }
