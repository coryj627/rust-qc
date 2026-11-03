#![allow(unused)]
use futures::StreamExt;
use h2_support::prelude::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use std::time::{Duration, Instant};
use tokio::time::{sleep, timeout};

struct C;
static LIVE: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for C {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 { LIVE.fetch_add(l.size(), SeqCst); System.alloc(l) }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) { LIVE.fetch_sub(l.size(), SeqCst); System.dealloc(p, l) }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 { LIVE.fetch_add(n, SeqCst); LIVE.fetch_sub(l.size(), SeqCst); System.realloc(p, l, n) }
}
#[global_allocator]
static A: C = C;

fn get() -> Request<()> { Request::builder().method("GET").uri("https://a.b/").body(()).unwrap() }

async fn barrier(srv: &mut mock::Handle) {
    // ping/pong so the client connection has processed everything sent so far
    srv.send_bytes(&[0,0,8,6,0,0,0,0,0, 9,9,9,9,9,9,9,9]).await;
    loop {
        match timeout(Duration::from_millis(500), srv.next()).await {
            Ok(Some(Ok(h2::frame::Frame::Ping(_)))) => break,
            Ok(Some(Ok(_))) => continue,
            o => panic!("barrier: {:?}", o.is_ok()),
        }
    }
}

async fn run(mode: &'static str, n: u32) -> usize {
    let (io, mut srv) = mock::new();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<client::ResponseFuture>();
    let (go_tx, mut go_rx) = tokio::sync::mpsc::unbounded_channel::<()>();
    let (done_tx, mut done_rx) = tokio::sync::mpsc::unbounded_channel::<()>();
    let cl = tokio::spawn(async move {
        let (mut c, h2) = client::handshake(io).await.unwrap();
        let conn = tokio::spawn(async move { let _ = h2.await; });
        while go_rx.recv().await.is_some() {
            let (mut resp, s) = c.send_request(get(), true).unwrap();
            tx.send(resp).unwrap();
            drop(s);
        }
        drop(conn);
    });
    srv.assert_client_handshake().await;
    let big = "v".repeat(8000);
    let mut base = 0;
    for i in 0..n {
        go_tx.send(()).unwrap();
        let resp = rx.recv().await.unwrap();
        let id = 2*i+1;
        let mut got = false;
        while !got { if let Some(Ok(h2::frame::Frame::Headers(_))) = srv.next().await { got = true } }
        if mode == "push-unpolled" {
            srv.send_frame(frames::push_promise(id, 2*i+2).request("GET", format!("https://a.b/{}?{}", i, big).as_str())).await;
        }
        srv.send_frame(frames::headers(id).response(200).field("x-big", big.clone()).eos()).await;
        barrier(&mut srv).await;
        match mode {
            "unpolled" | "push-unpolled" => drop(resp),
            _ => { let r = resp.await.unwrap(); drop(r); }
        }
        barrier(&mut srv).await;
        if i == 100 { base = LIVE.load(SeqCst); }
    }
    let end = LIVE.load(SeqCst);
    println!("  mode={mode}: live heap at req 100 = {base}, at req {n} = {end}, growth/req = {} bytes", (end - base) / (n as usize - 100));
    drop(go_tx);
    cl.abort();
    end - base
}

#[tokio::test]
async fn m18_response_future_drop_leak() {
    run("polled-control", 1100).await;
    run("unpolled", 1100).await;
}

#[tokio::test]
async fn m18b_push_leak() {
    run("push-unpolled", 1100).await;
}


fn raw(kind: u8, flags: u8, id: u32, payload: &[u8]) -> Vec<u8> {
    let mut v = vec![];
    let l = payload.len();
    v.extend_from_slice(&[(l >> 16) as u8, (l >> 8) as u8, l as u8, kind, flags]);
    v.extend_from_slice(&id.to_be_bytes());
    v.extend_from_slice(payload);
    v
}

async fn enc_run(name: &str, table: u32, n: u32) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use std::sync::Arc;
    let (mut a, b) = tokio::io::duplex(1 << 20);
    let done = Arc::new(AtomicUsize::new(0));
    let d2 = done.clone();
    let srv = tokio::spawn(async move {
        let mut conn = server::handshake(b).await.unwrap();
        let mut i = 0u32;
        while let Some(r) = conn.accept().await {
            let (_q, mut resp) = match r { Ok(x) => x, Err(_) => break };
            let rsp = Response::builder().header("x-request-id", format!("{:0>200}", i)).body(()).unwrap();
            resp.send_response(rsp, true).unwrap();
            i += 1;
            d2.store(i as usize, SeqCst);
        }
    });
    a.write_all(b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n").await.unwrap();
    let mut st = vec![0u8, 1];
    st.extend_from_slice(&table.to_be_bytes());
    a.write_all(&raw(4, 0, 0, &st)).await.unwrap();
    a.write_all(&raw(4, 1, 0, &[])).await.unwrap();
    let (mut rd, mut wr) = tokio::io::split(a);
    let dr = tokio::spawn(async move { let mut b = vec![0u8; 1 << 16]; loop { match rd.read(&mut b).await { Ok(0) | Err(_) => break, Ok(_) => {} } } });
    let req = [0x82, 0x87, 0x84, 0x01, 3, b'a', b'.', b'b'];
    sleep(Duration::from_millis(50)).await;
    let before = LIVE.load(SeqCst);
    let t = Instant::now();
    for k in 0..n {
        wr.write_all(&raw(1, 5, 2 * k + 1, &req)).await.unwrap();
        while done.load(SeqCst) + 50 < k as usize { tokio::task::yield_now().await; sleep(Duration::from_millis(1)).await; }
    }
    while done.load(SeqCst) < n as usize && t.elapsed() < Duration::from_secs(120) { sleep(Duration::from_millis(5)).await; }
    sleep(Duration::from_millis(200)).await;
    println!("  {name}: done {} live heap growth {} bytes after distinct 200-byte header values, took {:?}", done.load(SeqCst), LIVE.load(SeqCst) as i64 - before as i64, t.elapsed());
    srv.abort(); dr.abort();
    let _ = srv.await; 
    sleep(Duration::from_millis(50)).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn m26_encoder_unbounded() {
    enc_run("peer table 4096", 4096, 10000).await;
    enc_run("peer table 2^31-1", 0x7fffffff, 10000).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn m40_empty_data_flood() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let (mut a, b) = tokio::io::duplex(1 << 20);
    let srv = tokio::spawn(async move {
        let mut conn = server::handshake(b).await.unwrap();
        let mut keep = vec![];
        while let Some(r) = conn.accept().await { match r { Ok(x) => keep.push(x), Err(e) => { println!("  accept err {e}"); break } } }
    });
    a.write_all(b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n").await.unwrap();
    a.write_all(&raw(4, 0, 0, &[])).await.unwrap();
    a.write_all(&raw(4, 1, 0, &[])).await.unwrap();
    let (mut rd, mut wr) = tokio::io::split(a);
    let dr = tokio::spawn(async move { let mut b = vec![0u8; 1 << 16]; loop { match rd.read(&mut b).await { Ok(0) | Err(_) => break, Ok(_) => {} } } });
    let req = [0x82, 0x87, 0x84, 0x01, 3, b'a', b'.', b'b'];
    wr.write_all(&raw(1, 4, 1, &req)).await.unwrap();
    sleep(Duration::from_millis(100)).await;
    let before = LIVE.load(SeqCst);
    let frame = raw(0, 0, 1, &[]);
    let n = 500_000usize;
    let mut buf = Vec::with_capacity(9 * 10000);
    for i in 0..n {
        buf.extend_from_slice(&frame);
        if buf.len() >= 9 * 10000 { wr.write_all(&buf).await.unwrap(); buf.clear(); }
    }
    sleep(Duration::from_millis(500)).await;
    drop(buf);
    let g = LIVE.load(SeqCst) as i64 - before as i64;
    println!("  {n} empty DATA frames ({} wire bytes), body never read: live heap growth {g} bytes ({} bytes/frame)", n * 9, g / n as i64);
    srv.abort(); dr.abort();
}

#[tokio::test]
async fn m41_push_flood() {
    let (io, mut srv) = mock::new();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<usize>();
    let task = tokio::spawn(async move {
        let (mut c, h2) = client::handshake(io).await.unwrap();
        let c2 = c.clone();
        let (resp, _s) = c.send_request(get(), false).unwrap();
        let conn = tokio::spawn(async move { println!("  conn {:?}", h2.await.map_err(|e| e.to_string())) });
        while rx.recv().await.is_some() {}
        println!("  wired streams after flood: {}", c.num_wired_streams());
        drop(resp); drop(conn);
    });
    srv.assert_client_handshake_with_settings(frames::settings()).await;
    let _ = srv.next().await;
    let before = LIVE.load(SeqCst);
    let n = 20000u32;
    for i in 0..n {
        srv.send_frame(frames::push_promise(1, 2 * i + 2).request("GET", "https://a.b/x")).await;
        if i % 1000 == 0 { tokio::task::yield_now().await; }
    }
    sleep(Duration::from_millis(100)).await;
    println!("  {n} PUSH_PROMISE, default client: live heap growth {} bytes", LIVE.load(SeqCst) as i64 - before as i64);
    drop(tx);
    let _ = timeout(Duration::from_millis(500), task).await;
}
