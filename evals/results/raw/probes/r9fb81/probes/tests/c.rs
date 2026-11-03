use futures::{future::poll_fn, StreamExt};
use h2_support::prelude::*;
use std::time::Duration;
use tokio::time::timeout;

fn fr(ty: u8, flags: u8, id: u32, p: &[u8]) -> Vec<u8> {
    let l = p.len() as u32;
    let mut v = vec![(l >> 16) as u8, (l >> 8) as u8, l as u8, ty, flags];
    v.extend_from_slice(&id.to_be_bytes());
    v.extend_from_slice(p);
    v
}
fn lit(n: &[u8], val: &[u8]) -> Vec<u8> {
    let mut v = vec![0u8, n.len() as u8];
    v.extend_from_slice(n);
    v.push(val.len() as u8);
    v.extend_from_slice(val);
    v
}
/// GET https /  (static indexed) + authority
fn get_head() -> Vec<u8> {
    let mut v = vec![0x82u8, 0x87, 0x84];
    v.extend(lit(b":authority", b"a.test"));
    v
}
async fn drain(srv: &mut h2_support::mock::Handle, ms: u64) -> Vec<String> {
    let mut out = vec![];
    loop {
        match timeout(Duration::from_millis(ms), srv.next()).await {
            Ok(Some(Ok(f))) => out.push(format!("{:?}", f)),
            Ok(Some(Err(e))) => { out.push(format!("ERR {:?}", e)); break }
            Ok(None) => { out.push("EOF".into()); break }
            Err(_) => { out.push("idle".into()); break }
        }
    }
    out
}
fn req() -> Request<()> { Request::builder().uri("https://a.test/").body(()).unwrap() }


async fn pp_after_reset(name: &'static str, explicit: bool, cap0: bool) {
    let (io, mut srv) = mock::new();
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let (ptx, prx) = tokio::sync::oneshot::channel::<()>();
    let m = tokio::spawn(async move {
        srv.assert_client_handshake().await;
        let f = srv.next().await; println!("{} got {:?}", name, f);
        tx.send(()).unwrap();
        prx.await.unwrap();
        srv.send_frame(frames::push_promise(1, 2).request("GET", "https://a.test/s")).await;
        srv.send_frame(frames::headers(2).response(200).eos()).await;
        let f = drain(&mut srv, 300).await;
        println!("{} frames {:?}", name, f);
    });
    let mut b = client::Builder::new();
    if cap0 { b.max_concurrent_reset_streams(0); }
    let (mut c, mut conn) = b.handshake::<_, Bytes>(io).await.unwrap();
    let _ = timeout(Duration::from_millis(30), &mut conn).await;
    let (rf, mut st) = c.send_request(req(), false).unwrap();
    let _ = timeout(Duration::from_millis(50), &mut conn).await;
    rx.await.unwrap();
    if explicit { st.send_reset(Reason::CANCEL); }
    drop(rf); drop(st);
    if cap0 { let _ = timeout(Duration::from_millis(50), &mut conn).await; }
    ptx.send(()).unwrap();
    tokio::time::sleep(Duration::from_millis(30)).await;
    let r = timeout(Duration::from_millis(300), conn).await;
    println!("{} conn result {:?}", name, r);
    let _ = m.await; drop(c);
}
#[tokio::test] async fn p22_pp_after_scheduled_reset() { pp_after_reset("P22 implicit reset pending", false, false).await; }
#[tokio::test] async fn p23_pp_after_user_reset() { pp_after_reset("P23 explicit send_reset", true, false).await; }
#[tokio::test] async fn p23b_pp_after_forgotten() { pp_after_reset("P23b reset & forgotten", false, true).await; }

// P29 requests queued when EOF arrives
#[tokio::test]
async fn p29_pending_accept_at_eof() {
    let (io, mut srv) = mock::new();
    let m = tokio::spawn(async move {
        srv.assert_server_handshake().await;
        srv.send_frame(frames::headers(1).request("GET", "https://a.test/").eos()).await;
        srv.send_frame(frames::headers(3).request("GET", "https://a.test/2").eos()).await;
        srv.close_without_notify();
        drop(srv);
    });
    let mut conn = server::handshake(io).await.unwrap();
    m.await.unwrap();
    let r = conn.accept().await;
    println!("P29 first accept after peer write-close: {:?}", r.map(|x| x.map(|(q,_)| q.uri().clone())));
}
#[tokio::test]
async fn p29b_pending_accept_with_conn_error() {
    let (io, mut srv) = mock::new();
    let m = tokio::spawn(async move {
        srv.assert_server_handshake().await;
        srv.send_frame(frames::headers(1).request("GET", "https://a.test/").eos()).await;
        srv.send_bytes(&fr(6,0,0,&[0;7])).await; // connection error
        let f = drain(&mut srv, 200).await;
        println!("P29b frames {:?}", f);
    });
    let mut conn = server::handshake(io).await.unwrap();
    tokio::time::sleep(Duration::from_millis(30)).await;
    let r = conn.accept().await;
    println!("P29b accept: {:?}", r.map(|x| x.map(|(q,_)| q.uri().clone())));
    let _ = m.await;
}
// P30 dropped RecvStream holds connection window
#[tokio::test]
async fn p30_dropped_body_window() {
    let (io, mut srv) = mock::new();
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let m = tokio::spawn(async move {
        srv.assert_server_handshake().await;
        srv.send_frame(frames::headers(1).request("POST", "https://a.test/")).await;
        for _ in 0..4 { srv.send_frame(frames::data(1, vec![0u8; 16000])).await; }
        tokio::time::sleep(Duration::from_millis(50)).await;
        tx.send(()).unwrap();
        let f = drain(&mut srv, 300).await;
        println!("P30 frames after RecvStream drop (64000 buffered bytes): {:?}", f);
    });
    let mut conn = server::handshake(io).await.unwrap();
    let (r, mut resp) = conn.accept().await.unwrap().unwrap();
    let st = resp.send_response(Response::new(()), false).unwrap();
    let j = tokio::spawn(async move { let _ = timeout(Duration::from_millis(600), poll_fn(|cx| conn.poll_closed(cx))).await; });
    rx.await.unwrap();
    drop(r);
    let _ = m.await; let _ = j.await; drop(st); drop(resp);
}
// P32 RST(NO_ERROR) after complete response
#[tokio::test]
async fn p32_rst_no_error() {
    let (io, mut srv) = mock::new();
    let m = tokio::spawn(async move {
        srv.assert_client_handshake().await;
        let _ = srv.next().await;
        srv.send_frame(frames::headers(1).response(200)).await;
        srv.send_frame(frames::data(1, &b"hi"[..]).eos()).await;
        srv.send_frame(frames::reset(1).reason(Reason::NO_ERROR)).await;
        let f = drain(&mut srv, 200).await;
        println!("P32 frames {:?}", f);
    });
    let (mut c, conn) = client::handshake(io).await.unwrap();
    let (rf, mut st) = c.send_request(Request::builder().method("POST").uri("https://a.test/").body(()).unwrap(), false).unwrap();
    tokio::spawn(async move { let r = conn.await; println!("P32 conn {:?}", r); });
    tokio::time::sleep(Duration::from_millis(50)).await;
    let r = rf.await;
    println!("P32 response: {:?}", r.as_ref().map(|x| x.status()));
    let mut b = r.unwrap().into_body();
    println!("P32 is_end_stream before read {}", b.is_end_stream());
    println!("P32 data1 {:?}", b.data().await);
    println!("P32 data2 {:?}", b.data().await);
    println!("P32 trailers {:?}", b.trailers().await);
    let _ = m.await; let _ = &mut st;
}
// P33 trailers() with unread data
#[tokio::test]
async fn p33_trailers_with_data_queued() {
    let (io, mut srv) = mock::new();
    let m = tokio::spawn(async move {
        srv.assert_client_handshake().await;
        let _ = srv.next().await;
        srv.send_frame(frames::headers(1).response(200)).await;
        srv.send_frame(frames::data(1, &b"hi"[..])).await;
        tokio::time::sleep(Duration::from_millis(50)).await;
        srv.send_frame(frames::headers(1).field("grpc-status", "0").eos()).await;
        let f = drain(&mut srv, 200).await;
        println!("P33 frames {:?}", f);
    });
    let (mut c, conn) = client::handshake(io).await.unwrap();
    tokio::spawn(async move { let _ = conn.await; });
    let (rf, _st) = c.send_request(req(), true).unwrap();
    let mut b = rf.await.unwrap().into_body();
    tokio::time::sleep(Duration::from_millis(20)).await;
    let j = tokio::spawn(async move { let t = b.trailers().await; format!("{:?}", t) });
    println!("P33 trailers() with DATA unread: {:?}", timeout(Duration::from_millis(500), j).await);
    let _ = m.await;
}
// P34 pending-open stream hoards connection capacity
#[tokio::test]
async fn p34_pending_open_capacity() {
    let (io, mut srv) = mock::new();
    let m = tokio::spawn(async move {
        srv.assert_client_handshake_with_settings(frames::settings().max_concurrent_streams(1)).await;
        let mut got = 0; 
        loop { match timeout(Duration::from_millis(400), srv.next()).await {
            Ok(Some(Ok(h2::frame::Frame::Data(d)))) => { got += d.payload().len(); println!("P34 DATA on {:?} len {}", d.stream_id(), d.payload().len()); }
            Ok(Some(Ok(f))) => println!("P34 {:?}", f),
            _ => break } }
        println!("P34 DATA bytes received by server = {} of 1000", got);
    });
    let (c, conn) = client::handshake::<_>(io).await.unwrap();
    tokio::spawn(async move { let _ = conn.await; });
    let mut c = c.ready().await.unwrap();
    tokio::time::sleep(Duration::from_millis(30)).await;
    let (_r1, mut s1) = c.send_request(Request::builder().method("POST").uri("https://a.test/1").body(()).unwrap(), false).unwrap();
    tokio::time::sleep(Duration::from_millis(30)).await;
    let (_r2, mut s2) = c.send_request(Request::builder().method("POST").uri("https://a.test/2").body(()).unwrap(), false).unwrap();
    s2.reserve_capacity(65535);
    tokio::time::sleep(Duration::from_millis(30)).await;
    println!("P34 pending-open stream capacity after reserve: {}", s2.capacity());
    s1.send_data(Bytes::from(vec![0u8; 1000]), true).unwrap();
    let _ = m.await;
}
// P35 explicit reset of a stream before HEADERS go out
#[tokio::test]
async fn p35a_reset_before_flush() {
    let (io, mut srv) = mock::new();
    let m = tokio::spawn(async move {
        srv.assert_client_handshake().await;
        let f = drain(&mut srv, 300).await;
        println!("P35a wire frames: {:?}", f);
    });
    let (c, mut conn) = client::handshake(io).await.unwrap();
    let _ = timeout(Duration::from_millis(30), &mut conn).await;
    let mut c = c.ready().await.unwrap();
    let (r, mut s) = c.send_request(req(), false).unwrap();
    s.send_reset(Reason::CANCEL);
    let _ = timeout(Duration::from_millis(100), &mut conn).await;
    drop(r); drop(s);
    let _ = m.await;
}
#[tokio::test]
async fn p35b_reset_real_server_sees_idle_rst() {
    // client against real h2 server over duplex
    let (a, b) = tokio::io::duplex(65536);
    let srv = tokio::spawn(async move {
        let mut conn = server::handshake(b).await.unwrap();
        let r = conn.accept().await;
        format!("{:?}", r.map(|x| x.map(|_| ())))
    });
    let (c, mut conn) = client::handshake(a).await.unwrap();
    let _ = timeout(Duration::from_millis(30), &mut conn).await;
    let mut c = c.ready().await.unwrap();
    let (r, mut s) = c.send_request(req(), false).unwrap();
    s.send_reset(Reason::CANCEL);
    let res = timeout(Duration::from_millis(200), &mut conn).await;
    println!("P35b client conn {:?}", res);
    println!("P35b server accept: {:?}", timeout(Duration::from_millis(200), srv).await);
    drop(r); drop(s);
}
// P36 parent reset before PP flush
#[tokio::test]
async fn p36_push_lost_with_parent() {
    let (io, mut srv) = mock::new();
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let (atx, arx) = tokio::sync::oneshot::channel::<()>();
    let m = tokio::spawn(async move {
        srv.assert_server_handshake().await;
        srv.send_frame(frames::headers(1).request("GET", "https://a.test/")).await;
        tx.send(()).unwrap();
        arx.await.unwrap();
        srv.send_frame(frames::reset(1).cancel()).await;
        let f = drain(&mut srv, 300).await;
        println!("P36 frames {:?}", f);
    });
    let mut conn = server::handshake(io).await.unwrap();
    let (_r, mut resp) = conn.accept().await.unwrap().unwrap();
    rx.await.unwrap();
    let mut pushed = resp.push_request(Request::builder().uri("https://a.test/style.css").body(()).unwrap()).unwrap();
    atx.send(()).unwrap();
    tokio::time::sleep(Duration::from_millis(20)).await;
    let _ = timeout(Duration::from_millis(50), poll_fn(|cx| conn.poll_closed(cx))).await;
    let sr = pushed.send_response(Response::new(()), false);
    println!("P36 pushed.send_response after parent reset: {:?}", sr.as_ref().map(|_| "Ok").map_err(|e| e.to_string()));
    if let Ok(mut st) = sr {
        st.reserve_capacity(100);
        println!("P36 send_data {:?}", st.send_data(Bytes::from_static(b"body"), true).map_err(|e| e.to_string()));
        tokio::spawn(async move { let _ = timeout(Duration::from_millis(500), poll_fn(|cx| conn.poll_closed(cx))).await; });
        let t = tokio::spawn(poll_fn(move |cx| st.poll_reset(cx)));
        println!("P36 poll_reset on orphan pushed stream {:?}", timeout(Duration::from_millis(300), t).await);
    }
    let _ = m.await;
}
// P37 :protocol on non CONNECT
#[tokio::test]
async fn p37_protocol_get() {
    let (io, mut srv) = mock::new();
    let m = tokio::spawn(async move {
        srv.assert_client_handshake().await;
        let f = timeout(Duration::from_millis(200), srv.next()).await;
        if let Ok(Some(Ok(h2::frame::Frame::Headers(h)))) = f { println!("P37 wire: {:?}", h.into_parts().0); } else { println!("P37 wire {:?}", f.map(|x| x.map(|y| y.map(|_| ())))); }
    });
    let (mut c, conn) = client::handshake(io).await.unwrap();
    tokio::spawn(async move { let _ = conn.await; });
    let mut q = req(); q.extensions_mut().insert(Protocol::from("websocket"));
    println!("P37 send {:?}", c.send_request(q, true).as_ref().map(|_| "Ok").map_err(|e| e.to_string()));
    let _ = m.await;
}
// P39 header list size boundary
#[tokio::test]
async fn p39_list_size_boundary() {
    // :method GET(7+3+32=42) :scheme https(7+5+32=44) :path /(5+1+32=38) => 124 octets total
    for lim in [124u32, 125u32] {
        let (io, mut srv) = mock::new();
        let m = tokio::spawn(async move {
            srv.assert_server_handshake().await;
            srv.send_bytes(&fr(1,5,1,&[0x82,0x87,0x84])).await;
            let f = drain(&mut srv, 100).await;
            println!("P39 max_header_list_size={} frames {:?}", lim, f);
        });
        let mut b = server::Builder::new(); b.max_header_list_size(lim);
        let mut conn = b.handshake::<_, Bytes>(io).await.unwrap();
        let _ = timeout(Duration::from_millis(100), conn.accept()).await;
        let _ = m.await;
    }
}
// P31 quadratic re-decoding of a huge literal name across CONTINUATION frames
fn huff_a(n: usize) -> Vec<u8> {
    // 'a' = 00011 (5 bits)
    let mut out = Vec::with_capacity(n * 5 / 8 + 1);
    let mut acc = 0u64; let mut bits = 0;
    for _ in 0..n { acc = (acc << 5) | 0b00011; bits += 5; while bits >= 8 { bits -= 8; out.push((acc >> bits) as u8); } acc &= (1 << bits) - 1; }
    if bits > 0 { out.push(((acc << (8 - bits)) | ((1u64 << (8 - bits)) - 1)) as u8); }
    out
}
fn int7(flag: u8, mut v: usize) -> Vec<u8> {
    if v < 127 { return vec![flag | v as u8]; }
    let mut o = vec![flag | 127]; v -= 127;
    while v >= 128 { o.push(0x80 | (v as u8 & 0x7f)); v >>= 7; }
    o.push(v as u8); o
}
async fn huge_name(name_chars: usize, tail_frames: usize) -> std::time::Duration {
    let hn = huff_a(name_chars);
    let mut block = vec![0x82u8, 0x87, 0x84, 0x00];
    block.extend(int7(0x80, hn.len()));
    block.extend(&hn);
    // declare a value that never completes until the last frame
    let val_len = tail_frames * 16384;
    block.extend(int7(0x00, val_len));
    let mut all = vec![];
    let chunks: Vec<&[u8]> = block.chunks(16384).collect();
    let nfirst = chunks.len();
    for (i, c) in chunks.iter().enumerate() {
        let (ty, fl) = if i == 0 { (1u8, 1u8) } else { (9u8, 0u8) };
        all.extend(fr(ty, fl, 1, c));
    }
    for i in 0..tail_frames { all.extend(fr(9, if i + 1 == tail_frames { 4 } else { 0 }, 1, &vec![b'v'; 16384])); }
    println!("P31 input bytes {} frames {}", all.len(), nfirst + tail_frames);
    let (io, mut srv) = mock::new();
    let m = tokio::spawn(async move {
        srv.assert_server_handshake().await;
        srv.send_bytes(&all).await;
        let f = drain(&mut srv, 200).await;
        println!("P31 frames {:?}", f.iter().take(2).collect::<Vec<_>>());
    });
    let mut conn = server::handshake(io).await.unwrap();
    let t = std::time::Instant::now();
    let r = timeout(Duration::from_secs(12), conn.accept()).await;
    let e = t.elapsed();
    println!("P31 name_chars={} tail_frames={} elapsed {:?} accept {:?}", name_chars, tail_frames, e, r.map(|x| x.map(|y| y.map(|_| ()))));
    let _ = m.await;
    e
}
#[tokio::test(flavor = "multi_thread")]
async fn p31_quadratic() {
    huge_name(1 << 20, 1).await;
    huge_name(1 << 20, 50).await;
    huge_name(1 << 20, 100).await;
}
// P26b threshold
async fn many_headers_ok(n: usize) -> bool {
    let mut block = get_head();
    for i in 0..n { let nm = format!("h{:x}", i); block.extend(lit(nm.as_bytes(), b"")); }
    let chunks: Vec<&[u8]> = block.chunks(16384).collect();
    let mut all = vec![];
    for (i, c) in chunks.iter().enumerate() {
        let end = i + 1 == chunks.len();
        let (ty, fl) = if i == 0 { (1u8, if end {5} else {1}) } else { (9u8, if end {4} else {0}) };
        all.extend(fr(ty, fl, 1, c));
    }
    let (io, mut srv) = mock::new();
    let m = tokio::spawn(async move { srv.assert_server_handshake().await; srv.send_bytes(&all).await; let _ = drain(&mut srv, 100).await; });
    let j = tokio::spawn(async move { let mut conn = server::handshake(io).await.unwrap(); let _ = timeout(Duration::from_millis(300), conn.accept()).await; });
    let r = j.await.is_ok();
    m.abort(); let _ = m.await;
    r
}
#[tokio::test]
async fn p26b_threshold() {
    std::panic::set_hook(Box::new(|_| {}));
    let (mut lo, mut hi) = (24000usize, 25000usize);
    while hi - lo > 1 { let mid = (lo + hi) / 2; if many_headers_ok(mid).await { lo = mid } else { hi = mid } }
    println!("P26b largest ok count of regular fields = {}, panics at {}", lo, hi);
}
