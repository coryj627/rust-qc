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


// P3 fixed: outer timeout does not re-poll the inner future
#[tokio::test]
async fn p03a_poll_reset_not_woken() {
    let (io, mut srv) = mock::new();
    let m = tokio::spawn(async move {
        srv.assert_server_handshake().await;
        srv.send_frame(frames::headers(1).request("POST", "https://a.test/").field("content-length","1")).await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        srv.send_frame(frames::data(1, &b"xxxx"[..])).await;
        let f = drain(&mut srv, 500).await;
        println!("P03a frames: {:?}", f);
    });
    let mut conn = server::handshake(io).await.unwrap();
    let (_r, mut resp) = conn.accept().await.unwrap().unwrap();
    let t = tokio::spawn(poll_fn(move |cx| resp.poll_reset(cx)));
    tokio::spawn(async move { let _ = timeout(Duration::from_millis(1500), poll_fn(|cx| conn.poll_closed(cx))).await; });
    println!("P03a parked SendResponse::poll_reset: {:?}", timeout(Duration::from_millis(800), t).await);
    let _ = m.await;
}
#[tokio::test]
async fn p03b_poll_capacity_not_woken() {
    let (io, mut srv) = mock::new();
    let m = tokio::spawn(async move {
        srv.assert_server_handshake().await;
        srv.send_frame(frames::headers(1).request("GET", "https://a.test/")).await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        srv.send_frame(frames::window_update(1, 0x7fff_ffff)).await;
        let f = drain(&mut srv, 500).await;
        println!("P03b frames: {:?}", f);
    });
    let mut conn = server::handshake(io).await.unwrap();
    let (_r, mut resp) = conn.accept().await.unwrap().unwrap();
    let mut st = resp.send_response(Response::new(()), false).unwrap();
    st.reserve_capacity(1 << 20);
    tokio::spawn(async move { let _ = timeout(Duration::from_millis(1500), poll_fn(|cx| conn.poll_closed(cx))).await; });
    let first = poll_fn(|cx| st.poll_capacity(cx)).await;
    println!("P03b first capacity {:?}", first);
    let t = tokio::spawn(async move { let r = poll_fn(|cx| st.poll_capacity(cx)).await; format!("{:?}", r) });
    println!("P03b parked poll_capacity: {:?}", timeout(Duration::from_millis(800), t).await);
    let _ = m.await;
}
#[tokio::test]
async fn p03c_poll_reset_stream_not_woken() {
    let (io, mut srv) = mock::new();
    let m = tokio::spawn(async move {
        srv.assert_server_handshake().await;
        srv.send_frame(frames::headers(1).request("GET", "https://a.test/")).await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        srv.send_frame(frames::window_update(1, 0x7fff_ffff)).await;
        let f = drain(&mut srv, 500).await;
        println!("P03c frames: {:?}", f);
    });
    let mut conn = server::handshake(io).await.unwrap();
    let (_r, mut resp) = conn.accept().await.unwrap().unwrap();
    let mut st = resp.send_response(Response::new(()), false).unwrap();
    tokio::spawn(async move { let _ = timeout(Duration::from_millis(1500), poll_fn(|cx| conn.poll_closed(cx))).await; });
    let t = tokio::spawn(poll_fn(move |cx| st.poll_reset(cx)));
    println!("P03c parked SendStream::poll_reset: {:?}", timeout(Duration::from_millis(800), t).await);
    let _ = m.await;
}

// P10: remote resets of pending pushes never leave the counter
#[tokio::test]
async fn p10_push_reset_counter() {
    let (io, mut srv) = mock::new();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(1);
    let (atx, mut arx) = tokio::sync::mpsc::channel::<()>(1);
    let m = tokio::spawn(async move {
        srv.assert_client_handshake().await;
        let _ = srv.next().await;
        srv.send_frame(frames::headers(1).response(200)).await;
        for i in 0..30u32 {
            let id = 2 + 2 * i;
            srv.send_frame(frames::push_promise(1, id).request("GET", "https://a.test/s")).await;
            srv.send_frame(frames::reset(id).cancel()).await;
            srv.send_frame(frames::ping([1,2,3,4,5,6,7,8])).await;
            // wait for the pong so the reset has been processed before the client pops the promise
            loop { match srv.next().await { Some(Ok(h2::frame::Frame::Ping(_))) => break,
                Some(Ok(f)) => { println!("P10 iter {} frame {:?}", i, f); if matches!(f, h2::frame::Frame::GoAway(_)) { return; } }
                o => { println!("P10 iter {} end {:?}", i, o); return; } } }
            if tx.send(()).await.is_err() { return; }
            let _ = arx.recv().await;
        }
        println!("P10 survived 30 promised+reset, none outstanding");
    });
    let (mut c, conn) = client::handshake(io).await.unwrap();
    tokio::spawn(async move { let r = conn.await; println!("P10 conn result {:?}", r); });
    let (mut rf, _) = c.send_request(req(), true).unwrap();
    let mut pp = rf.push_promises();
    let _resp = rf.await.unwrap();
    let mut n = 0;
    while rx.recv().await.is_some() {
        let p = pp.push_promise().await;
        n += 1;
        if n <= 1 || p.is_none() { println!("P10 popped {}: {:?}", n, p.as_ref().map(|x| x.is_ok())); }
        drop(p);
        let _ = atx.send(()).await;
    }
    println!("P10 client popped {} promises", n);
    let _ = m.await;
}
// P13 push after final response
#[tokio::test]
async fn p13_push_after_end() {
    let (io, mut srv) = mock::new();
    let m = tokio::spawn(async move {
        srv.assert_server_handshake().await;
        srv.send_frame(frames::headers(1).request("GET", "https://a.test/").eos()).await;
        let f = drain(&mut srv, 300).await;
        println!("P13 frames: {:?}", f);
    });
    let mut conn = server::handshake(io).await.unwrap();
    let (_r, mut resp) = conn.accept().await.unwrap().unwrap();
    resp.send_response(Response::new(()), true).unwrap();
    let p = resp.push_request(Request::builder().uri("https://a.test/late").body(()).unwrap());
    println!("P13 push_request after final response: {:?}", p.as_ref().map(|_| "Ok").map_err(|e| e.to_string()));
    let _ = timeout(Duration::from_millis(300), poll_fn(|cx| conn.poll_closed(cx))).await;
    let _ = m.await; drop(p);
}
// P14 failed push leaves a slot behind
#[tokio::test]
async fn p14_failed_push_leak() {
    let (io, mut srv) = mock::new();
    let m = tokio::spawn(async move {
        srv.assert_server_handshake().await;
        srv.send_frame(frames::headers(1).request("GET", "https://a.test/").eos()).await;
        let f = drain(&mut srv, 300).await;
        println!("P14 frames: {:?}", f);
    });
    let mut conn = server::handshake(io).await.unwrap();
    let (_r, mut resp) = conn.accept().await.unwrap().unwrap();
    let before = conn.num_wired_streams();
    for _ in 0..100 {
        let e = resp.push_request(Request::builder().method("POST").uri("https://a.test/x").body(()).unwrap());
        assert!(e.is_err());
    }
    println!("P14 wired streams before {} after 100 failed pushes {}", before, conn.num_wired_streams());
    resp.send_response(Response::new(()), true).unwrap();
    let _ = timeout(Duration::from_millis(200), poll_fn(|cx| conn.poll_closed(cx))).await;
    println!("P14 wired streams after response: {}", conn.num_wired_streams());
    let _ = m.await;
}
// P21 duplicate RST on pending-accept stream double counts
#[tokio::test]
async fn p21_double_rst() {
    let (io, mut srv) = mock::new();
    let m = tokio::spawn(async move {
        srv.assert_server_handshake().await;
        srv.send_frame(frames::headers(1).request("GET", "https://a.test/")).await;
        for _ in 0..25 { srv.send_frame(frames::reset(1).cancel()).await; }
        let f = drain(&mut srv, 300).await;
        println!("P21 frames: {:?}", f);
    });
    let mut conn = server::handshake(io).await.unwrap();
    let r = timeout(Duration::from_millis(300), poll_fn(|cx| conn.poll_closed(cx))).await;
    println!("P21 conn: {:?}", r);
    let _ = m.await;
}
// P5c pushed HEAD with content-length
#[tokio::test]
async fn p05c_pushed_head() {
    let (io, mut srv) = mock::new();
    let m = tokio::spawn(async move {
        srv.assert_client_handshake().await;
        let _ = srv.next().await;
        srv.send_frame(frames::headers(1).response(200)).await;
        srv.send_frame(frames::push_promise(1, 2).request("HEAD", "https://a.test/s")).await;
        srv.send_frame(frames::headers(2).response(200).field("content-length", "10").eos()).await;
        srv.send_frame(frames::data(1, &b""[..]).eos()).await;
        let f = drain(&mut srv, 200).await;
        println!("P05c frames {:?}", f);
    });
    let (mut c, conn) = client::handshake(io).await.unwrap();
    tokio::spawn(async move { let r = conn.await; println!("P05c conn {:?}", r); });
    let (mut rf, _) = c.send_request(req(), true).unwrap();
    let mut pp = rf.push_promises();
    let _resp = rf.await.unwrap();
    let p = pp.push_promise().await.unwrap().unwrap();
    let (_q, r) = p.into_parts();
    println!("P05c pushed HEAD response: {:?}", r.await.map(|x| x.status()));
    let _ = m.await;
}
// P22/23 PUSH_PROMISE on a stream the client has reset
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
    let (rf, mut st) = c.send_request(req(), false).unwrap();
    // drive until HEADERS are out, then stop polling
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

// P24 / P25 remote-triggered assertions on pushed streams
#[tokio::test]
async fn p24_push_concurrency_assert() {
    let (io, mut srv) = mock::new();
    let m = tokio::spawn(async move {
        srv.assert_client_handshake().await;
        let _ = srv.next().await;
        srv.send_frame(frames::headers(1).response(200)).await;
        srv.send_frame(frames::push_promise(1, 2).request("GET", "https://a.test/a")).await;
        srv.send_frame(frames::push_promise(1, 4).request("GET", "https://a.test/b")).await;
        srv.send_frame(frames::headers(2).response(200)).await;
        srv.send_frame(frames::headers(4).response(200)).await;
        let f = drain(&mut srv, 200).await;
        println!("P24 frames {:?}", f);
    });
    let mut b = client::Builder::new(); b.max_concurrent_streams(1);
    let (mut c, conn) = b.handshake::<_, Bytes>(io).await.unwrap();
    let (rf, _) = c.send_request(req(), true).unwrap();
    let j = tokio::spawn(conn);
    let _ = timeout(Duration::from_millis(200), rf).await;
    println!("P24 conn task: {:?}", timeout(Duration::from_millis(300), j).await);
    let _ = m.await;
}
#[tokio::test]
async fn p25_push_double_1xx_assert() {
    let (io, mut srv) = mock::new();
    let m = tokio::spawn(async move {
        srv.assert_client_handshake().await;
        let _ = srv.next().await;
        srv.send_frame(frames::headers(1).response(200)).await;
        srv.send_frame(frames::push_promise(1, 2).request("GET", "https://a.test/a")).await;
        srv.send_frame(frames::headers(2).response(100)).await;
        srv.send_frame(frames::headers(2).response(103)).await;
        let f = drain(&mut srv, 200).await;
        println!("P25 frames {:?}", f);
    });
    let (mut c, conn) = client::handshake(io).await.unwrap();
    let (rf, _) = c.send_request(req(), true).unwrap();
    let j = tokio::spawn(conn);
    let _ = timeout(Duration::from_millis(200), rf).await;
    println!("P25 conn task: {:?}", timeout(Duration::from_millis(300), j).await);
    let _ = m.await;
}
// P26 many distinct header fields -> HeaderMap capacity
async fn many_headers(n: usize, conn_frames: usize) {
    let mut block = get_head();
    for i in 0..n { let nm = format!("h{:x}", i); block.extend(lit(nm.as_bytes(), b"")); }
    let chunks: Vec<&[u8]> = block.chunks(16384).collect();
    println!("P26 n={} header block bytes={} frames={}", n, block.len(), chunks.len());
    let _ = conn_frames;
    let mut all = vec![];
    for (i, c) in chunks.iter().enumerate() {
        let end = i + 1 == chunks.len();
        let (ty, fl) = if i == 0 { (1u8, if end {5} else {1}) } else { (9u8, if end {4} else {0}) };
        all.extend(fr(ty, fl, 1, c));
    }
    let (io, mut srv) = mock::new();
    let m = tokio::spawn(async move {
        srv.assert_server_handshake().await;
        srv.send_bytes(&all).await;
        let f = drain(&mut srv, 300).await;
        println!("P26 frames {:?}", f);
    });
    let j = tokio::spawn(async move {
        let mut conn = server::handshake(io).await.unwrap();
        let r = timeout(Duration::from_millis(500), conn.accept()).await;
        match r { Ok(Some(Ok((q, _)))) => format!("accepted with {} headers", q.headers().len()),
                  o => format!("{:?}", o.map(|x| x.map(|y| y.map(|_| ())))) }
    });
    println!("P26 server task: {:?}", j.await);
    m.abort(); let _ = m.await;
}
#[tokio::test] async fn p26_many_headers() { many_headers(20000, 0).await; many_headers(40000, 0).await; }

// P27 client sends malformed requests
async fn client_send(name: &str, r: Request<()>) {
    let (io, mut srv) = mock::new();
    let nm = name.to_string();
    let m = tokio::spawn(async move {
        srv.assert_client_handshake().await;
        let f = timeout(Duration::from_millis(200), srv.next()).await;
        match f { Ok(Some(Ok(h2::frame::Frame::Headers(h)))) => {
            let (p, flds) = h.into_parts();
            println!("{} wire pseudo {:?} fields {:?}", nm, p, flds);
        } o => println!("{} wire {:?}", nm, o.map(|x| x.map(|y| y.map(|_| ())))) }
    });
    let (mut c, conn) = client::handshake(io).await.unwrap();
    tokio::spawn(async move { let _ = conn.await; });
    let res = c.send_request(r, true);
    println!("{} send_request {:?}", name, res.as_ref().map(|_| "Ok").map_err(|e| e.to_string()));
    let _ = m.await;
}
#[tokio::test]
async fn p27_client_malformed() {
    client_send("P27a GET authority-form", Request::builder().uri("a.test:80").body(()).unwrap()).await;
    client_send("P27b CONNECT relative", Request::builder().method("CONNECT").uri("/foo").body(()).unwrap()).await;
    client_send("P27c GET relative, HTTP/1.1 default", Request::builder().uri("/foo").body(()).unwrap()).await;
    client_send("P27d two te values", Request::builder().uri("https://a.test/").header("te","trailers").header("te","gzip").body(()).unwrap()).await;
    client_send("P27e host header and authority", Request::builder().uri("https://a.test/").header("host","b.test").body(()).unwrap()).await;
    client_send("P27f OPTIONS *", Request::builder().method("OPTIONS").uri("*").version(Version::HTTP_2).body(()).unwrap()).await;
}
// P28 send_trailers with connection header; send_response 101; reserve_capacity truncation
#[tokio::test]
async fn p28_send_side() {
    let (io, mut srv) = mock::new();
    let m = tokio::spawn(async move {
        srv.assert_server_handshake().await;
        srv.send_frame(frames::headers(1).request("GET", "https://a.test/").eos()).await;
        loop { match timeout(Duration::from_millis(200), srv.next()).await {
            Ok(Some(Ok(h2::frame::Frame::Headers(h)))) => { let e = h.is_end_stream(); let (p, f) = h.into_parts(); println!("P28 HEADERS pseudo={:?} fields={:?} eos={}", p.status, f, e); }
            Ok(Some(Ok(f))) => println!("P28 {:?}", f),
            _ => break } }
    });
    let mut conn = server::handshake(io).await.unwrap();
    let (_r, mut resp) = conn.accept().await.unwrap().unwrap();
    let mut st = resp.send_response(Response::new(()), false).unwrap();
    st.reserve_capacity(1usize << 32);
    println!("P28 reserve_capacity(1<<32) capacity after poll: requested");
    let cap = timeout(Duration::from_millis(100), poll_fn(|cx| st.poll_capacity(cx))).await;
    println!("P28 poll_capacity after reserve_capacity(1<<32): {:?} capacity()={}", cap.map(|x| x.map(|y| y.is_ok())), st.capacity());
    let mut t = HeaderMap::new(); t.insert("connection", "close".parse().unwrap());
    println!("P28 send_trailers(connection: close): {:?}", st.send_trailers(t).map_err(|e| e.to_string()));
    let _ = timeout(Duration::from_millis(200), poll_fn(|cx| conn.poll_closed(cx))).await;
    let _ = m.await;
}
