// Structured frame fuzzer: drives a real h2 server or client over an in-memory duplex with random
// frames and random user API calls, using a no-op waker, and tallies panic locations.
use bytes::Bytes;
use h2::{client, server, RecvStream, SendStream};
use http::{HeaderMap, Request, Response};
use std::collections::BTreeMap;
use std::future::Future;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};
use tokio::io::{AsyncRead, AsyncWrite, DuplexStream, ReadBuf};

struct N; impl Wake for N { fn wake(self: Arc<Self>) {} }
fn w() -> Waker { Waker::from(Arc::new(N)) }

struct R(u64);
impl R { fn n(&mut self) -> u64 { let mut x = self.0; x ^= x << 13; x ^= x >> 7; x ^= x << 17; self.0 = x; x }
  fn b(&mut self, m: u64) -> u64 { self.n() % m } }

fn fr(ty: u8, flags: u8, id: u32, p: &[u8]) -> Vec<u8> {
    let l = p.len() as u32;
    let mut v = vec![(l >> 16) as u8, (l >> 8) as u8, l as u8, ty, flags];
    v.extend_from_slice(&id.to_be_bytes()); v.extend_from_slice(p); v
}
fn lit(pre: u8, n: &[u8], val: &[u8]) -> Vec<u8> {
    let mut v = vec![pre, n.len() as u8]; v.extend_from_slice(n); v.push(val.len() as u8); v.extend_from_slice(val); v
}
fn block(r: &mut R, server_side: bool) -> Vec<u8> {
    let mut v = vec![];
    if r.b(8) == 0 { v.push(0x20 | r.b(31) as u8); }
    if server_side {
        match r.b(8) { 0 => {}, 1 => v.extend(lit(0, b":method", b"CONNECT")), _ => v.push(0x82 + r.b(2) as u8) }
        if r.b(8) != 0 { v.push(0x86 + r.b(2) as u8); }
        if r.b(8) != 0 { v.push(0x84); }
        if r.b(4) != 0 { v.extend(lit(0, b":authority", b"a.test")); }
        if r.b(16) == 0 { v.extend(lit(0, b":protocol", b"ws")); }
    } else {
        match r.b(8) {
            0 => {}, 1 => v.extend(lit(0, b":status", b"100")), 2 => v.extend(lit(0, b":status", b"204")),
            3 => v.extend(lit(0, b":status", b"304")), 4 => v.extend(lit(0, b":status", b"101")),
            _ => v.push(0x88) }
    }
    for _ in 0..r.b(4) {
        match r.b(14) {
            0 => v.extend(lit(0, b"content-length", b"5")),
            1 => v.extend(lit(0, b"content-length", b"0")),
            2 => v.extend(lit(0x40, b"x-a", b"1")),
            3 => v.push(0x80 | (62 + r.b(3) as u8)),
            4 => v.extend(lit(0, b"te", b"trailers")),
            5 => v.extend(lit(0, b"te", b"gzip")),
            6 => v.extend(lit(0, b"connection", b"x")),
            7 => { if std::env::var_os("GENTLE").is_none() { v.extend(lit(0, b"x-b", b"\n")) } },
            8 => v.push(0x80 | r.b(61) as u8 + 1),
            9 => v.extend(lit(0x10, b"x-c", b"z")),
            10 => { if std::env::var_os("GENTLE").is_none() { let n = r.b(4) as usize; for _ in 0..n { v.push(r.n() as u8); } } },
            11 => v.push(0x88),
            12 => v.extend(lit(0x40, b":path", b"/q")),
            _ => v.extend(lit(0, b"foo", b"bar")),
        }
    }
    v
}
fn frame(r: &mut R, server_side: bool) -> Vec<u8> {
    let ids: &[u32] = if std::env::var_os("GENTLE").is_some() { if server_side { &[1, 1, 3, 3, 5, 7] } else { &[1, 1, 1, 3, 2, 2, 4, 4, 6] } } else { &[1, 1, 1, 3, 3, 5, 7, 2, 4, 6, 9, 0] };
    let id = ids[r.b(ids.len() as u64) as usize];
    match r.b(20) {
        0..=4 => { let mut b = block(r, server_side);
            let mut flags = 0x4u8;
            if r.b(2) == 0 { flags |= 1; }
            if r.b(8) == 0 && std::env::var_os("GENTLE").is_none() { flags &= !4; }
            if r.b(16) == 0 { flags |= 0x20; let mut x = vec![0, 0, 0, r.b(5) as u8, 3]; x.append(&mut b); b = x; }
            if r.b(16) == 0 { flags |= 0x8; let mut x = vec![1]; x.append(&mut b); x.push(0); b = x; }
            fr(1, flags, id, &b) }
        5..=8 => { let n = [0usize, 1, 5, 100, 16384][r.b(5) as usize]; let mut flags = 0; if r.b(2) == 0 { flags |= 1 } 
            let mut p = vec![0u8; n]; if r.b(8) == 0 { flags |= 8; if !p.is_empty() { p[0] = r.b(n as u64) as u8; } }
            fr(0, flags, id, &p) }
        9 => { let mut p = vec![]; p.extend_from_slice(&(r.b(14) as u32).to_be_bytes()); fr(3, 0, id, &p) }
        10..=11 => { let mut p = vec![]; for _ in 0..1 + r.b(2) { let (k, val): (u16, u32) = match r.b(8) {
                0 => (1, [0u32, 100, 4096, 65536][r.b(4) as usize]),
                1 => (2, r.b(2) as u32),
                2 => (3, [0u32, 1, 2, 100][r.b(4) as usize]),
                3 => (4, [0u32, 1, 100, 65535, 1 << 20, 0x7fff_ffff][r.b(6) as usize]),
                4 => (5, [16384u32, 20000][r.b(2) as usize]),
                5 => (6, [100u32, 1000][r.b(2) as usize]),
                6 => (8, r.b(2) as u32),
                _ => (99, 1) };
                p.extend_from_slice(&k.to_be_bytes()); p.extend_from_slice(&val.to_be_bytes()); }
            fr(4, if r.b(6) == 0 { 1 } else { 0 }, 0, if r.b(6) == 0 { &[] } else { &p }) }
        12..=13 => { let inc = [1u32, 100, 65535, 1 << 20, 0x7fff_ffff][r.b(5) as usize]; fr(8, 0, if r.b(3) == 0 { 0 } else { id }, &inc.to_be_bytes()) }
        14 => fr(6, r.b(2) as u8, 0, &[r.n() as u8 & 1; 8]),
        15..=16 => { let pid = [2u32, 2, 4, 6, 8, 10][r.b(6) as usize]; let mut p = pid.to_be_bytes().to_vec(); p.extend(block(r, true));
            let mut fl = 4; if r.b(8) == 0 { fl = 0 } fr(5, fl, id, &p) }
        17 if std::env::var_os("GENTLE").is_some() => fr(6, 0, 0, &[0; 8]),
        17 => { let last = [0u32, 1, 3, 0x7fff_ffff][r.b(4) as usize]; let mut p = last.to_be_bytes().to_vec(); p.extend_from_slice(&(r.b(3) as u32).to_be_bytes()); fr(7, 0, 0, &p) }
        18 if std::env::var_os("GENTLE").is_some() => fr(6, 0, 0, &[0; 8]),
        18 => { let b = block(r, server_side); fr(9, if r.b(4) == 0 { 0 } else { 4 }, id, &b) }
        _ => fr(2, 0, id, &[0, 0, 0, r.b(4) as u8, 1]),
    }
}
fn pump(io: &mut DuplexStream, cx: &mut Context, out: &mut Vec<u8>) {
    let mut buf = [0u8; 8192];
    loop { let mut rb = ReadBuf::new(&mut buf);
        match Pin::new(&mut *io).poll_read(cx, &mut rb) { Poll::Ready(Ok(())) if !rb.filled().is_empty() => {}, _ => break } }
    while !out.is_empty() { match Pin::new(&mut *io).poll_write(cx, out) { Poll::Ready(Ok(n)) if n > 0 => { out.drain(..n); } _ => break } }
}
const PRE: &[u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";

fn run_server(seed: u64) {
    let mut r = R(seed | 1);
    let waker = w(); let mut cx = Context::from_waker(&waker);
    let (io, mut peer) = tokio::io::duplex(1 << 20);
    let mut b = server::Builder::new();
    match r.b(4) { 0 => { b.max_concurrent_reset_streams(r.b(3) as usize); } 1 => { b.max_concurrent_streams(1 + r.b(2) as u32); } 2 => { b.reset_stream_duration(std::time::Duration::from_millis(0)); } _ => {} }
    if r.b(4) == 0 { b.max_header_list_size(200); }
    if r.b(4) == 0 { b.enable_connect_protocol(); }
    let mut hs = Box::pin(b.handshake::<_, Bytes>(io));
    let mut out = PRE.to_vec(); out.extend(fr(4, 0, 0, &[])); out.extend(fr(4, 1, 0, &[]));
    let mut conn = loop { pump(&mut peer, &mut cx, &mut out);
        if let Poll::Ready(c) = hs.as_mut().poll(&mut cx) { match c { Ok(c) => break c, Err(_) => return } } };
    let mut bodies: Vec<RecvStream> = vec![]; let mut resp: Vec<server::SendResponse<Bytes>> = vec![];
    let mut sends: Vec<SendStream<Bytes>> = vec![]; let mut pushed: Vec<server::SendPushedResponse<Bytes>> = vec![];
    let steps = 5 + r.b(60);
    for _ in 0..steps {
        if r.b(2) == 0 { let f = frame(&mut r, true); if !(std::env::var_os("GENTLE").is_some() && f[3] == 5) { out.extend(f); } }
        pump(&mut peer, &mut cx, &mut out);
        for _ in 0..3 { match conn.poll_accept(&mut cx) { Poll::Ready(Some(Ok((q, s)))) => { bodies.push(q.into_body()); resp.push(s); } Poll::Ready(Some(Err(_))) | Poll::Ready(None) => {} Poll::Pending => break } }
        match r.b(16) {
            0 => { if let Some(mut s) = resp.pop() { let eos = r.b(2) == 0; let st = [200u16, 100, 204, 101][r.b(4) as usize];
                if let Ok(x) = s.send_response(Response::builder().status(st).body(()).unwrap(), eos) { sends.push(x); } resp.push(s); } }
            1 if std::env::var_os("NOPUSH").is_some() => {}
            1 => { if let Some(s) = resp.last_mut() { if let Ok(p) = s.push_request(Request::builder().uri("https://a.test/p").body(()).unwrap()) { pushed.push(p); } } }
            2 => { if let Some(mut p) = pushed.pop() { if r.b(3) != 0 { if let Ok(x) = p.send_response(Response::new(()), r.b(2) == 0) { sends.push(x); } } } }
            3 => { if !resp.is_empty() { let i = r.b(resp.len() as u64) as usize; resp.remove(i); } }
            4 => { if !bodies.is_empty() { let i = r.b(bodies.len() as u64) as usize; bodies.remove(i); } }
            5 => { if !sends.is_empty() { let i = r.b(sends.len() as u64) as usize; sends.remove(i); } }
            6 => { if let Some(s) = sends.last_mut() { let n = [0usize, 10, 70000][r.b(3) as usize]; let _ = s.send_data(Bytes::from(vec![0u8; n]), r.b(3) == 0); } }
            7 => { if let Some(s) = sends.last_mut() { s.reserve_capacity([0usize, 100, 1 << 20][r.b(3) as usize]); let _ = s.poll_capacity(&mut cx); } }
            8 => { if let Some(s) = sends.last_mut() { let _ = s.send_trailers(HeaderMap::new()); } }
            9 => { if let Some(s) = sends.last_mut() { s.send_reset(h2::Reason::CANCEL); } }
            10 => { if let Some(s) = resp.last_mut() { s.send_reset(h2::Reason::INTERNAL_ERROR); } }
            11 => { if let Some(b) = bodies.last_mut() { if let Poll::Ready(Some(Ok(d))) = b.poll_data(&mut cx) { let n = d.len(); let _ = b.flow_control().release_capacity(n); } } }
            12 => { if let Some(b) = bodies.last_mut() { let _ = b.poll_trailers(&mut cx); let _ = b.is_end_stream(); } }
            13 => { if let Some(s) = resp.last_mut() { let _ = s.poll_reset(&mut cx); } if let Some(s) = sends.last_mut() { let _ = s.poll_reset(&mut cx); let _ = s.capacity(); } }
            14 => { match r.b(6) { 0 => conn.graceful_shutdown(), 1 => conn.abrupt_shutdown(h2::Reason::NO_ERROR), 2 => { let _ = conn.set_initial_window_size(r.b(100000) as u32); } 3 => conn.set_target_window_size(r.b(100000) as u32), _ => {} } }
            _ => {}
        }
        let _ = conn.poll_closed(&mut cx);
    }
    drop((bodies, resp, sends, pushed));
    for _ in 0..3 { pump(&mut peer, &mut cx, &mut out); let _ = conn.poll_closed(&mut cx); }
}
fn run_client(seed: u64) {
    let mut r = R(seed | 1);
    let waker = w(); let mut cx = Context::from_waker(&waker);
    let (io, mut peer) = tokio::io::duplex(1 << 20);
    let mut b = client::Builder::new();
    match r.b(4) { 0 => { b.max_concurrent_reset_streams(r.b(3) as usize); } 1 => { b.max_concurrent_streams(1 + r.b(2) as u32); } 2 => { b.reset_stream_duration(std::time::Duration::from_millis(0)); } _ => {} }
    if r.b(4) == 0 { b.max_header_list_size(200); }
    if r.b(6) == 0 { b.initial_max_send_streams(1); }
    if std::env::var_os("NOPUSH").is_some() { b.enable_push(false); }
    let mut hs = Box::pin(b.handshake::<_, Bytes>(io));
    let mut out = fr(4, 0, 0, &[]); out.extend(fr(4, 1, 0, &[]));
    let (mut sr, mut conn) = loop { pump(&mut peer, &mut cx, &mut out);
        if let Poll::Ready(c) = hs.as_mut().poll(&mut cx) { match c { Ok(c) => break c, Err(_) => return } } };
    let mut rfs: Vec<client::ResponseFuture> = vec![]; let mut sends: Vec<SendStream<Bytes>> = vec![]; let mut bodies: Vec<RecvStream> = vec![];
    let mut pps: Vec<client::PushPromises> = vec![]; let mut prs: Vec<client::PushedResponseFuture> = vec![];
    let steps = 5 + r.b(60);
    for _ in 0..steps {
        if r.b(2) == 0 { let f = frame(&mut r, false); if !(std::env::var_os("NOPUSH").is_some() && f[3] == 5) { if std::env::var_os("REPLAY").is_some() { eprintln!("FRAME len={} ty={} fl={:#x} id={} {:02x?}", f.len()-9, f[3], f[4], u32::from_be_bytes([f[5],f[6],f[7],f[8]]), &f[9..f.len().min(40)]); } out.extend(f); } }
        pump(&mut peer, &mut cx, &mut out);
        let _ = Pin::new(&mut conn).poll(&mut cx);
        let act = r.b(14); if std::env::var_os("REPLAY").is_some() { eprintln!("ACT {}", act); }
        match act {
            0 | 1 => { if let Poll::Ready(Ok(())) = sr.poll_ready(&mut cx) { let m = ["GET", "POST", "HEAD", "CONNECT"][r.b(4) as usize];
                let u = if m == "CONNECT" { "a.test:80" } else { "https://a.test/" };
                if let Ok((mut f, s)) = sr.send_request(Request::builder().method(m).uri(u).body(()).unwrap(), r.b(2) == 0) { if r.b(2) == 0 { pps.push(f.push_promises()); } rfs.push(f); sends.push(s); } } }
            2 => { if !rfs.is_empty() { let i = r.b(rfs.len() as u64) as usize; let mut f = rfs.remove(i); match Pin::new(&mut f).poll(&mut cx) { Poll::Ready(Ok(x)) => bodies.push(x.into_body()), Poll::Ready(Err(_)) => {} Poll::Pending => { if r.b(3) != 0 { rfs.push(f) } } } } }
            3 => { if let Some(p) = pps.last_mut() { if let Poll::Ready(Some(Ok(x))) = p.poll_push_promise(&mut cx) { let (_, f) = x.into_parts(); prs.push(f); } } }
            4 => { if !prs.is_empty() { let i = r.b(prs.len() as u64) as usize; let mut f = prs.remove(i); match Pin::new(&mut f).poll(&mut cx) { Poll::Ready(Ok(x)) => bodies.push(x.into_body()), Poll::Ready(Err(_)) => {} Poll::Pending => { if r.b(3) != 0 { prs.push(f) } } } } }
            5 => { if !sends.is_empty() { let i = r.b(sends.len() as u64) as usize; sends.remove(i); } }
            6 => { if !bodies.is_empty() { let i = r.b(bodies.len() as u64) as usize; bodies.remove(i); } }
            7 => { if !pps.is_empty() { let i = r.b(pps.len() as u64) as usize; pps.remove(i); } }
            8 => { if let Some(s) = sends.last_mut() { let n = [0usize, 10, 70000][r.b(3) as usize]; let _ = s.send_data(Bytes::from(vec![0u8; n]), r.b(3) == 0); } }
            9 => { if let Some(s) = sends.last_mut() { s.reserve_capacity([0usize, 100, 1 << 20][r.b(3) as usize]); let _ = s.poll_capacity(&mut cx); } }
            10 => { if let Some(s) = sends.last_mut() { if r.b(2) == 0 { s.send_reset(h2::Reason::CANCEL) } else { let _ = s.send_trailers(HeaderMap::new()); } } }
            11 => { if let Some(b) = bodies.last_mut() { if let Poll::Ready(Some(Ok(d))) = b.poll_data(&mut cx) { let n = d.len(); let _ = b.flow_control().release_capacity(n); } } }
            12 => { if let Some(b) = bodies.last_mut() { let _ = b.poll_trailers(&mut cx); } }
            _ => { if r.b(4) == 0 { let _ = conn.set_initial_window_size(r.b(100000) as u32); } }
        }
    }
    drop((rfs, sends, bodies, pps, prs));
    for _ in 0..3 { pump(&mut peer, &mut cx, &mut out); let _ = Pin::new(&mut conn).poll(&mut cx); }
}
fn main() {
    if let Ok(x) = std::env::var("REPLAY") { let seed: u64 = x.parse().unwrap(); if seed & 1 == 0 { run_server(seed >> 1) } else { run_client(seed >> 1) } return; }
    let secs: u64 = std::env::args().nth(1).unwrap().parse().unwrap();
    let base: u64 = std::env::args().nth(2).map(|s| s.parse().unwrap()).unwrap_or(0x9E3779B97F4A7C15);
    let tally: Arc<Mutex<BTreeMap<String, (u64, u64, String)>>> = Default::default();
    thread_local! { static SEED: std::cell::Cell<u64> = const { std::cell::Cell::new(0) }; }
    let t2 = tally.clone();
    std::panic::set_hook(Box::new(move |i| {
        let loc = i.location().map(|l| format!("{}:{}", l.file().replace("/work/crate/", ""), l.line())).unwrap_or_default();
        let msg = i.payload().downcast_ref::<String>().cloned().or_else(|| i.payload().downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
        let seed = SEED.with(|s| s.get());
        if let Ok(mut m) = t2.lock() { if !m.contains_key(&loc) { eprintln!("NEW {} seed={} panicking_already={} msg={}", loc, seed, std::thread::panicking(), msg.chars().take(160).collect::<String>()); }
            let e = m.entry(loc).or_insert((0, seed, msg.chars().take(160).collect())); e.0 += 1; }
    }));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(secs);
    let mut hs = vec![]; 
    for th in 0..4u64 { hs.push(std::thread::spawn(move || {
        let mut r = R(base.wrapping_mul(th * 2 + 1) | 1); let mut iters = 0u64;
        while std::time::Instant::now() < deadline { let seed = r.n(); SEED.with(|s| s.set(seed));
            let srv = seed & 1 == 0; 
            let _ = catch_unwind(AssertUnwindSafe(|| if srv { run_server(seed >> 1) } else { run_client(seed >> 1) }));
            iters += 1; }
        iters })); }
    let total: u64 = hs.into_iter().map(|h| h.join().unwrap_or(0)).sum();
    println!("iterations {}", total);
    for (k, v) in tally.lock().unwrap().iter() { println!("{} count={} seed={} msg={}", k, v.0, v.1, v.2); }
}
