use probe::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicIsize, Ordering::*};
struct C;
static LIVE: AtomicIsize = AtomicIsize::new(0);
unsafe impl GlobalAlloc for C {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 { LIVE.fetch_add(l.size() as isize, Relaxed); System.alloc(l) }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) { LIVE.fetch_sub(l.size() as isize, Relaxed); System.dealloc(p, l) }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 { LIVE.fetch_add(n as isize - l.size() as isize, Relaxed); System.realloc(p, l, n) }
}
#[global_allocator]
static A: C = C;
fn live() -> isize { LIVE.load(Relaxed) }
async fn settle() { for _ in 0..8 { tokio::task::yield_now().await; } sleep(Duration::from_millis(1)).await; for _ in 0..8 { tokio::task::yield_now().await; } }
async fn slab(polled: bool) {
    let mut x = cli(client::Builder::new()).await;
    let big = "v".repeat(4096);
    let mut base = 0;
    let n = 2000u32;
    for i in 0..n {
        let id = 2 * i + 1;
        let mut c = x.c.clone().ready().await.unwrap();
        let (r, _) = c.send_request(req_get("https://a.test/"), true).unwrap();
        settle().await;
        let _ = dump(&mut x.s, 1).await;
        x.s.send_bytes(&f(HEADERS, EH | ES, id, &probe::resp("200", &[("x-big", &big)]))).await;
        settle().await;
        if polled { let _ = r.await; } else { drop(r); }
        settle().await;
        if i == 200 { base = live(); }
    }
    let d = live() - base;
    println!("slab polled={polled}: grew {d} bytes over {} requests = {} B/req; wired={}", n - 201, d / (n as isize - 201), x.c.num_wired_streams());
}
#[tokio::test(flavor = "current_thread")]
async fn slab_unpolled() { slab(false).await; }
#[tokio::test(flavor = "current_thread")]
async fn slab_polled() { slab(true).await; }
#[tokio::test(flavor = "current_thread")]
async fn empty_data() {
    let (io, mut c) = mock::new();
    let j = tokio::spawn(async move {
        let mut conn = server::handshake(io).await.unwrap();
        let (_rq, _sr) = conn.accept().await.unwrap().unwrap();
        while let Some(_) = conn.accept().await {}
    });
    c.assert_server_handshake().await;
    c.send_bytes(&f(HEADERS, EH, 1, &req(&[]))).await;
    sleep(Duration::from_millis(20)).await;
    let b0 = live();
    let n = 200_000;
    let one = f(DATA, 0, 1, b"");
    for i in 0..n {
        c.send_bytes(&one).await;
        if i % 100 == 0 { settle().await; }
    }
    sleep(Duration::from_millis(100)).await;
    let d = live() - b0;
    println!("empty data: {} frames ({} wire bytes) grew heap {} = {} B/frame", n, n * 9, d, d / n as isize);
    let _ = dump(&mut c, 20).await;
    j.abort();
}
async fn table(sz: u32) {
    use tokio::io::AsyncReadExt;
    let (io, mut c) = mock::new();
    let j = tokio::spawn(async move {
        let mut conn = server::handshake(io).await.unwrap();
        let mut i = 0u64;
        while let Some(r) = conn.accept().await {
            let (_rq, mut sr) = r.unwrap();
            i += 1;
            let v = format!("{:01000}", i);
            let rs = Response::builder().header("x-uniq", v).body(()).unwrap();
            let _ = sr.send_response(rs, true);
        }
    });
    c.write_preface().await;
    let mut st = h2::frame::Settings::default();
    st.set_header_table_size(Some(sz));
    c.send(st.into()).await.unwrap();
    c.send_bytes(&probe::f(4, 1, 0, &[])).await;
    let mut buf = vec![0u8; 1 << 20];
    let n = 3000u32;
    let mut base = 0;
    for k in 0..n {
        c.send_bytes(&f(HEADERS, EH | ES, 2 * k + 1, &req(&[]))).await;
        settle().await;
        let _ = timeout(Duration::from_millis(1), c.read(&mut buf)).await;
        if k == 200 { base = live(); }
    }
    let d = live() - base;
    println!("table {sz}: grew {d} over {} responses = {} B/resp", n - 201, d / (n as isize - 201));
    j.abort();
}
#[tokio::test(flavor = "current_thread")]
async fn table_max() { table(u32::MAX).await; }
#[tokio::test(flavor = "current_thread")]
async fn table_4096() { table(4096).await; }
