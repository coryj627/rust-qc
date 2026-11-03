use h2_support::prelude::*;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

static SEEN: OnceLock<Mutex<HashMap<String, (u64, u64)>>> = OnceLock::new();
static CUR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

struct R(u64);
impl R {
    fn n(&mut self) -> u64 { self.0 ^= self.0 << 13; self.0 ^= self.0 >> 7; self.0 ^= self.0 << 17; self.0 }
    fn below(&mut self, m: u64) -> u64 { self.n() % m }
    fn pick<T: Copy>(&mut self, xs: &[T]) -> T { xs[self.below(xs.len() as u64) as usize] }
}
fn fr(kind: u8, flags: u8, id: u32, p: &[u8]) -> Vec<u8> {
    let mut v = vec![(p.len() >> 16) as u8, (p.len() >> 8) as u8, p.len() as u8, kind, flags];
    v.extend_from_slice(&id.to_be_bytes());
    v.extend_from_slice(p);
    v
}
fn lit(n: &str, v: &str) -> Vec<u8> {
    let mut o = vec![0, n.len() as u8];
    o.extend_from_slice(n.as_bytes());
    o.push(v.len() as u8);
    o.extend_from_slice(v.as_bytes());
    o
}
fn block(r: &mut R, server: bool) -> Vec<u8> {
    let mut b = Vec::new();
    match r.below(6) {
        0 => {}
        1 => b.push(0x88),
        2 => { b.push(r.pick(&[0x82, 0x83])); b.push(0x87); b.push(0x84); }
        3 => { b.push(0x88); b.extend(lit("content-length", r.pick(&["0", "5", "100000", ""]))); }
        4 => { b.extend(vec![0x08, 3]); b.extend_from_slice(r.pick(&[b"100", b"103", b"204", b"304", b"200"])); }
        _ => { b.push(0x82); b.push(0x87); b.extend(vec![1, 1, b'x']); b.push(0x84); }
    }
    let _ = server;
    for _ in 0..r.below(3) {
        match r.below(6) {
            0 => b.extend(lit("x", "y")),
            1 => b.extend(lit("te", "trailers")),
            2 => b.extend(lit("connection", "x")),
            3 => { b.push(0x40); b.extend(&lit("dyn", "v")[1..]); }
            4 => b.push(0x20 | r.below(5) as u8),
            _ => b.push(0xbe),
        }
    }
    b
}
fn gen(r: &mut R, server: bool) -> Vec<u8> {
    let id = r.pick(&[1u32, 1, 1, 3, 3, 5, 7, 2, 4, 6, 0, 9]);
    match r.below(16) {
        0 | 1 | 2 | 3 => fr(1, r.pick(&[4, 5, 4, 5, 0, 1, 0x24, 0xc]), id, &{
            let mut b = block(r, server);
            if r.below(10) == 0 { b.insert(0, 0); }
            b
        }),
        4 | 5 | 6 => {
            let n = r.pick(&[0usize, 1, 100, 5000, 16384]);
            let mut p = vec![b'z'; n];
            let fl = r.pick(&[0, 1, 8, 9]);
            if fl & 8 != 0 { let pad = r.below(50) as u8; p.insert(0, pad); p.extend(vec![0; pad as usize]); }
            fr(0, fl, id, &p)
        }
        7 => fr(3, 0, id, &(r.pick(&[0u32, 1, 2, 7, 8, 11])).to_be_bytes()),
        8 | 9 => {
            let mut p = vec![];
            for _ in 0..1 + r.below(2) {
                p.extend_from_slice(&(r.pick(&[1u16, 2, 3, 4, 5, 6, 8])).to_be_bytes());
                p.extend_from_slice(&(r.pick(&[0u32, 1, 100, 4096, 16384, 65535, 100000, 0x7fff_ffff, 0xffff_ffff, 2000000])).to_be_bytes());
            }
            fr(4, r.pick(&[0, 0, 0, 1]), 0, if r.below(8) == 0 { &[] } else { &p })
        }
        10 => fr(6, r.pick(&[0, 1]), 0, &[0, 0, 0, 0, 0, 0, 0, r.below(3) as u8]),
        11 => { let mut p = (r.pick(&[0u32, 1, 3, 5, 0x7fff_ffff])).to_be_bytes().to_vec(); p.extend_from_slice(&(r.pick(&[0u32, 2, 11])).to_be_bytes()); fr(7, 0, 0, &p) }
        12 | 13 => fr(8, 0, r.pick(&[0u32, 1, 3, 2, 4, 5]), &(r.pick(&[1u32, 100, 65535, 1 << 20, 0x7fff_ffff, 0])).to_be_bytes()),
        14 => {
            let mut p = (r.pick(&[2u32, 4, 6, 8, 3, 0])).to_be_bytes().to_vec();
            p.push(0x82); p.push(0x87); p.extend(vec![1, 1, b'x']); p.push(0x84);
            fr(5, r.pick(&[4, 4, 0]), r.pick(&[1u32, 3, 5]), &p)
        }
        _ => match r.below(5) { 0 => fr(9, 4, 1, &[0x88]), 1 => fr(9, 0, 1, &[]), 2 => fr(2, 0, 1, &[0, 0, 0, 1, 16]), 3 => fr(0xa, 0, 1, &[1, 2]), _ => fr(2, 0, id, &[0, 0, 0, 1, 0]) },
    }
}
async fn server_case(seed: u64) {
    let mut r = R(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1);
    let (io, mut c) = mock::new();
    let appseed = r.n();
    let app = tokio::spawn(async move {
        let mut rr = R(appseed | 1);
        let mut srv = match server::Builder::new().max_concurrent_streams(rr.pick(&[1, 2, 100])).handshake::<_, Bytes>(io).await { Ok(s) => s, Err(_) => return };
        while let Some(res) = srv.accept().await {
            let (req, mut resp) = match res { Ok(x) => x, Err(_) => break };
            let mode = rr.below(6);
            tokio::spawn(async move {
                let mut body = req.into_body();
                if mode == 0 { drop(body); drop(resp); return; }
                if mode >= 2 { if let Some(Ok(b)) = body.data().await { let _ = body.flow_control().release_capacity(b.len()); } }
                let rs = if mode == 5 { Response::builder().status(100).body(()).unwrap() } else { Response::new(()) };
                if let Ok(mut st) = resp.send_response(rs, mode == 1) {
                    if mode == 3 { if let Ok(p) = resp.push_request(Request::builder().uri("https://x.com/p").body(()).unwrap()) { if mode == 3 { drop(p); } } }
                    st.reserve_capacity(100000);
                    let _ = st.send_data(Bytes::from(vec![0u8; 70000]), mode == 4);
                    tokio::task::yield_now().await;
                    if mode == 2 { let mut t = HeaderMap::new(); t.insert("x", "y".parse().unwrap()); let _ = st.send_trailers(t); }
                    if mode == 3 { st.send_reset(Reason::CANCEL); }
                }
            });
        }
    });
    let feed = async {
        tokio::time::timeout(Duration::from_millis(200), c.assert_server_handshake()).await.ok();
        for _ in 0..4 + r.below(40) {
            let f = gen(&mut r, true);
            c.send_bytes(&f).await;
            for _ in 0..3 { tokio::task::yield_now().await; }
        }
        for _ in 0..5 { tokio::task::yield_now().await; }
    };
    feed.await;
    drop(c);
    let _ = tokio::time::timeout(Duration::from_millis(100), app).await;
}

async fn client_case(seed: u64) {
    let mut r = R(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1);
    let (io, mut c) = mock::new();
    let appseed = r.n();
    let app = tokio::spawn(async move {
        let mut rr = R(appseed | 1);
        let (mut client, conn) = match client::Builder::new().max_concurrent_streams(rr.pick(&[1, 2, 100])).handshake::<_, Bytes>(io).await { Ok(s) => s, Err(_) => return };
        tokio::spawn(async move { let _ = conn.await; });
        for i in 0..3 {
            let mode = rr.below(4);
            let req = Request::builder().method(if i == 2 { "HEAD" } else { "POST" }).uri("https://x.com/").body(()).unwrap();
            let Ok(mut client2) = client.clone().ready().await else { return };
            let Ok((mut resp, mut st)) = client2.send_request(req, false) else { return };
            let mut pp = resp.push_promises();
            tokio::spawn(async move {
                if mode == 0 { drop(pp); drop(resp); drop(st); return; }
                st.reserve_capacity(100000);
                let _ = st.send_data(Bytes::from(vec![0u8; 70000]), mode == 1);
                if mode == 2 {
                    tokio::spawn(async move {
                        use futures::StreamExt;
                        while let Some(Ok(p)) = pp.next().await {
                            let (_rq, rs) = p.into_parts();
                            if let Ok(x) = rs.await { let mut b = x.into_body(); while let Some(Ok(_)) = b.data().await {} }
                        }
                    });
                }
                if let Ok(x) = resp.await {
                    let mut b = x.into_body();
                    while let Some(Ok(d)) = b.data().await { let _ = b.flow_control().release_capacity(d.len()); }
                    let _ = b.trailers().await;
                }
                drop(st);
            });
        }
    });
    let feed = async {
        tokio::time::timeout(Duration::from_millis(200), c.assert_client_handshake()).await.ok();
        for _ in 0..4 + r.below(40) {
            let f = gen(&mut r, false);
            c.send_bytes(&f).await;
            for _ in 0..3 { tokio::task::yield_now().await; }
        }
        for _ in 0..5 { tokio::task::yield_now().await; }
    };
    feed.await;
    drop(c);
    let _ = tokio::time::timeout(Duration::from_millis(100), app).await;
}

#[test]
fn fuzz() {
    let secs: u64 = std::env::var("FUZZ_SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(60);
    SEEN.set(Mutex::new(HashMap::new())).unwrap();
    std::panic::set_hook(Box::new(|info| {
        let loc = info.location().map(|l| format!("{}:{}", l.file(), l.line())).unwrap_or_default();
        let msg = info.payload().downcast_ref::<String>().cloned().or_else(|| info.payload().downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
        let msg: String = msg.chars().take(160).collect();
        let k = format!("{loc} {msg}");
        let seed = CUR.load(std::sync::atomic::Ordering::Relaxed);
        let mut g = SEEN.get().unwrap().lock().unwrap();
        let e = g.entry(k).or_insert((0, seed));
        e.0 += 1;
    }));
    let start = Instant::now();
    let mut n = 0u64;
    let mut seed = std::env::var("FUZZ_START").ok().and_then(|s| s.parse().ok()).unwrap_or(1u64);
    while start.elapsed().as_secs() < secs {
        CUR.store(seed, std::sync::atomic::Ordering::Relaxed);
        let rt = tokio::runtime::Builder::new_current_thread().enable_time().build().unwrap();
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            rt.block_on(async { if seed % 2 == 0 { server_case(seed).await } else { client_case(seed).await } });
        }));
        drop(rt);
        seed += 1; n += 1;
    }
    let g = SEEN.get().unwrap().lock().unwrap();
    println!("cases: {n}");
    let mut v: Vec<_> = g.iter().collect();
    v.sort_by_key(|(_, (c, _))| std::cmp::Reverse(*c));
    for (k, (c, s)) in v { println!("{c:8} first_seed={s} {k}"); }
}
