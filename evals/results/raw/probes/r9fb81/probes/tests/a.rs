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

// P1: padded DATA and flow control
#[tokio::test]
async fn p01_padded_flow_control() {
    let (io, mut srv) = mock::new();
    let m = tokio::spawn(async move {
        let s = srv.assert_client_handshake().await; let _ = s;
        let _ = srv.next().await; // HEADERS
        srv.send_frame(frames::headers(1).response(200)).await;
        // each frame: 1 pad-length byte, 1 data byte, 254 padding => 256 octets of flow control
        let mut p = vec![254u8, b'x']; p.extend(vec![0u8; 254]);
        for _ in 0..300 { srv.send_bytes(&fr(0, 0x8, 1, &p)).await; }
        let frames = drain(&mut srv, 300).await;
        println!("P01 frames seen by sender after 300 padded frames (76800 flow-controlled octets, window 65535): {:?}", frames);
        srv
    });
    let (mut c, conn) = client::handshake(io).await.unwrap();
    tokio::spawn(async move { let r = conn.await; println!("P01 conn result {:?}", r); });
    let (rf, _) = c.send_request(Request::builder().uri("https://a.test/").body(()).unwrap(), true).unwrap();
    let mut body = rf.await.unwrap().into_body();
    let mut n = 0;
    while let Ok(Some(d)) = timeout(Duration::from_millis(200), body.data()).await {
        let d = d.unwrap(); n += d.len();
        body.flow_control().release_capacity(d.len()).unwrap();
    }
    println!("P01 client got {} body bytes, no flow control error", n);
    let _ = m.await;
}

// P2: pushed stream handle dropped before PUSH_PROMISE flush
async fn push_drop(builder: server::Builder) {
    let (io, mut srv) = mock::new();
    let m = tokio::spawn(async move {
        srv.assert_server_handshake().await;
        srv.send_frame(frames::headers(1).request("GET", "https://a.test/").eos()).await;
        let f = drain(&mut srv, 300).await;
        println!("P02 frames: {:?}", f);
    });
    let mut conn = builder.handshake::<_, Bytes>(io).await.unwrap();
    let (r, mut resp) = conn.accept().await.unwrap().unwrap();
    drop(r);
    let pushed = resp.push_request(Request::builder().uri("https://a.test/style.css").body(()).unwrap()).unwrap();
    drop(pushed);
    let mut rr = Response::builder().status(200).body(()).unwrap();
    let _ = &mut rr;
    resp.send_response(rr, true).unwrap();
    let r = timeout(Duration::from_millis(300), poll_fn(|cx| conn.poll_closed(cx))).await;
    println!("P02 conn: {:?}", r);
    let _ = m.await;
}
#[tokio::test]
async fn p02a_push_drop_default() { push_drop(server::Builder::new()).await; }
#[tokio::test]
async fn p02b_push_drop_zero_reset_cap() {
    let mut b = server::Builder::new(); b.max_concurrent_reset_streams(0);
    push_drop(b).await;
}

// P3: library reset does not wake send task
#[tokio::test]
async fn p03a_poll_reset_not_woken() {
    let (io, mut srv) = mock::new();
    let m = tokio::spawn(async move {
        srv.assert_server_handshake().await;
        srv.send_frame(frames::headers(1).request("POST", "https://a.test/").field("content-length","1")).await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        srv.send_frame(frames::data(1, &b"xxxx"[..])).await; // violates content-length
        let f = drain(&mut srv, 500).await;
        println!("P03a frames: {:?}", f);
    });
    let mut conn = server::handshake(io).await.unwrap();
    let (_r, mut resp) = conn.accept().await.unwrap().unwrap();
    let t = tokio::spawn(async move {
        let r = timeout(Duration::from_millis(400), poll_fn(move |cx| resp.poll_reset(cx))).await;
        println!("P03a poll_reset parked task result: {:?}", r);
    });
    let _ = timeout(Duration::from_millis(600), poll_fn(|cx| conn.poll_closed(cx))).await;
    let _ = t.await; let _ = m.await;
}
#[tokio::test]
async fn p03b_poll_capacity_not_woken() {
    let (io, mut srv) = mock::new();
    let m = tokio::spawn(async move {
        srv.assert_server_handshake().await;
        srv.send_frame(frames::headers(1).request("GET", "https://a.test/")).await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        srv.send_frame(frames::window_update(1, 0x7fff_ffff)).await; // overflow -> stream error
        let f = drain(&mut srv, 500).await;
        println!("P03b frames: {:?}", f);
    });
    let mut conn = server::handshake(io).await.unwrap();
    let (_r, mut resp) = conn.accept().await.unwrap().unwrap();
    let mut st = resp.send_response(Response::new(()), false).unwrap();
    st.reserve_capacity(1 << 20);
    let t = tokio::spawn(async move {
        let first = poll_fn(|cx| st.poll_capacity(cx)).await;
        println!("P03b first capacity {:?}", first);
        let r = timeout(Duration::from_millis(400), poll_fn(|cx| st.poll_capacity(cx))).await;
        println!("P03b poll_capacity parked task result: {:?}", r);
    });
    let _ = timeout(Duration::from_millis(600), poll_fn(|cx| conn.poll_closed(cx))).await;
    let _ = t.await; let _ = m.await;
}

// P4: connection capacity leak via implicit reset with buffered data and big reservation
#[tokio::test]
async fn p04_capacity_leak() {
    let (io, mut srv) = mock::new();
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let m = tokio::spawn(async move {
        srv.assert_client_handshake_with_settings(frames::settings().initial_window_size(0)).await;
        let f1 = srv.next().await; println!("P04 got {:?}", f1); // HEADERS 1
        tokio::time::sleep(Duration::from_millis(50)).await;
        srv.send_frame(frames::window_update(1, 60000)).await;
        let f = drain(&mut srv, 100).await;
        println!("P04 stream1 frames: {:?}", f);
        tx.send(()).unwrap();
        let f = srv.next().await; println!("P04 got {:?}", f); // HEADERS 3
        srv.send_frame(frames::window_update(3, 65535)).await;
        let mut got = 0usize;
        loop {
            match timeout(Duration::from_millis(300), srv.next()).await {
                Ok(Some(Ok(h2::frame::Frame::Data(d)))) => { got += d.payload().len(); }
                Ok(o) => { println!("P04 other {:?}", o); }
                Err(_) => break,
            }
        }
        println!("P04 stream3 asked to send 20000, connection window advertises 65525 unused; DATA bytes delivered = {}", got);
    });
    let (mut c, conn) = client::handshake(io).await.unwrap();
    tokio::spawn(async move { let _ = conn.await; });
    let mut c = c.clone().ready().await.unwrap();
    let (rf, mut st) = c.send_request(Request::builder().method("POST").uri("https://a.test/").body(()).unwrap(), false).unwrap();
    st.reserve_capacity(60000);
    st.send_data(Bytes::from_static(b"0123456789"), false).unwrap();
    tokio::time::sleep(Duration::from_millis(20)).await;
    drop(st); drop(rf);
    rx.await.unwrap();
    let (rf3, mut st3) = c.send_request(Request::builder().method("POST").uri("https://a.test/").body(()).unwrap(), false).unwrap();
    st3.send_data(Bytes::from(vec![0u8; 20000]), false).unwrap();
    let _ = m.await; drop(rf3);
}

// P5
async fn client_resp(name: &str, send: impl FnOnce(&mut h2_support::mock::Handle) -> std::pin::Pin<Box<dyn std::future::Future<Output=()> + Send + '_>> + Send + 'static) {
    let (io, mut srv) = mock::new();
    let nm = name.to_string();
    let m = tokio::spawn(async move {
        srv.assert_client_handshake().await;
        let _ = srv.next().await;
        send(&mut srv).await;
        let f = drain(&mut srv, 200).await;
        println!("{} frames: {:?}", nm, f);
    });
    let (mut c, conn) = client::handshake(io).await.unwrap();
    tokio::spawn(async move { let r = conn.await; println!("conn {:?}", r); });
    let (rf, _) = c.send_request(req(), true).unwrap();
    let r = timeout(Duration::from_millis(200), rf).await;
    match r {
        Ok(Ok(resp)) => {
            let st = resp.status(); let hd = resp.headers().clone();
            let mut b = resp.into_body(); let mut n = 0; let mut err = None;
            while let Ok(Some(d)) = timeout(Duration::from_millis(100), b.data()).await {
                match d { Ok(d) => n += d.len(), Err(e) => { err = Some(e); break } }
            }
            println!("{}: status={} headers={:?} body_len={} err={:?}", name, st, hd, n, err);
        }
        o => println!("{}: result {:?}", name, o.map(|x| x.map(|_| ()))),
    }
    let _ = m.await;
}
#[tokio::test]
async fn p05a_1xx_content_length() {
    client_resp("P05a 100+cl then 200 w/o cl, 10-byte body", |s| Box::pin(async move {
        s.send_frame(frames::headers(1).response(100).field("content-length","5")).await;
        s.send_frame(frames::headers(1).response(200)).await;
        s.send_frame(frames::data(1,&b"0123456789"[..]).eos()).await;
    })).await;
}
#[tokio::test]
async fn p05b_304_content_length() {
    client_resp("P05b 304 cl=100 then empty DATA ES", |s| Box::pin(async move {
        s.send_frame(frames::headers(1).response(304).field("content-length","100")).await;
        s.send_frame(frames::data(1,&b""[..]).eos()).await;
    })).await;
}
#[tokio::test]
async fn p06_missing_status() {
    client_resp("P06 HEADERS without :status", |s| Box::pin(async move {
        s.send_bytes(&fr(1, 0x5, 1, &lit(b"foo", b"bar"))).await;
    })).await;
}
#[tokio::test]
async fn p06b_request_pseudo_in_response() {
    client_resp("P06b response with :path and :status", |s| Box::pin(async move {
        let mut p = vec![0x88u8]; p.extend(lit(b":path", b"/evil"));
        s.send_bytes(&fr(1, 0x5, 1, &p)).await;
    })).await;
}
#[tokio::test]
async fn p06c_pseudo_in_trailers() {
    client_resp("P06c trailers with :status", |s| Box::pin(async move {
        s.send_frame(frames::headers(1).response(200)).await;
        s.send_bytes(&fr(1, 0x5, 1, &[0x88u8])).await;
    })).await;
}
#[tokio::test]
async fn p16_bad_hpack_index() {
    client_resp("P16 invalid HPACK index", |s| Box::pin(async move {
        s.send_bytes(&fr(1, 0x5, 1, &[0xffu8, 0x80, 0x01])).await;
    })).await;
}
#[tokio::test]
async fn p08b_client_bad_value_kills_conn() {
    client_resp("P08b response with \\n in header value", |s| Box::pin(async move {
        let mut p = vec![0x88u8]; p.extend(lit(b"x", b"a\nb"));
        s.send_bytes(&fr(1, 0x5, 1, &p)).await;
    })).await;
}

// server side raw request
async fn srv_raw(name: &str, bytes: Vec<u8>) { srv_raw_b(name, bytes, server::Builder::new()).await }
async fn srv_raw_b(name: &str, bytes: Vec<u8>, b: server::Builder) {
    let (io, mut srv) = mock::new();
    let nm = name.to_string();
    let m = tokio::spawn(async move {
        srv.assert_server_handshake().await;
        srv.send_bytes(&bytes).await;
        let f = drain(&mut srv, 300).await;
        println!("{} frames: {:?}", nm, f);
    });
    let mut conn = b.handshake::<_, Bytes>(io).await.unwrap();
    loop {
        match timeout(Duration::from_millis(300), conn.accept()).await {
            Ok(Some(Ok((r, mut resp)))) => {
                println!("{}: ACCEPTED {} {} {:?} ext_ver={:?}", name, r.method(), r.uri(), r.headers(), r.version());
                let mut body = r.into_body();
                let mut n = 0; let mut e = None;
                while let Ok(Some(d)) = timeout(Duration::from_millis(50), body.data()).await {
                    match d { Ok(d) => n += d.len(), Err(x) => { e = Some(x); break } }
                }
                println!("{}: body {} err {:?} trailers {:?}", name, n, e, timeout(Duration::from_millis(50), body.trailers()).await);
                let _ = resp.send_response(Response::new(()), true);
            }
            Ok(Some(Err(e))) => { println!("{}: accept err {:?}", name, e); break }
            Ok(None) => { println!("{}: accept None", name); break }
            Err(_) => { println!("{}: accept idle", name); break }
        }
    }
    let _ = m.await;
}
#[tokio::test]
async fn p07_missing_path() {
    srv_raw("P07a GET no :path no :authority", fr(1,5,1,&[0x82,0x87])).await;
    let mut p = vec![0x82u8,0x87]; p.extend(lit(b":authority", b"a.test"));
    srv_raw("P07b GET no :path w/ :authority", fr(1,5,1,&p)).await;
    let mut p = lit(b":method", b"CONNECT");
    srv_raw("P07c CONNECT no :authority", fr(1,5,1,&p)).await;
    p = vec![0x82u8,0x87]; p.extend(lit(b":path", b"foo")); p.extend(lit(b":authority", b"a.test"));
    srv_raw("P07d :path foo", fr(1,5,1,&p)).await;
    p = vec![0x82u8]; p.extend(lit(b":path", b"/x"));
    srv_raw("P07e no :scheme", fr(1,5,1,&p)).await;
    p = vec![0x82u8,0x87,0x84]; p.extend(lit(b":authority", b"a.test")); p.extend(lit(b"host", b"b.test"));
    srv_raw("P07f authority!=host", fr(1,5,1,&p)).await;
    p = vec![0x82u8,0x87,0x84]; p.extend(lit(b":authority", b"")); 
    srv_raw("P07g empty authority", fr(1,5,1,&p)).await;
    p = vec![0x82u8,0x87]; p.extend(lit(b":path", b"*")); p.extend(lit(b":authority", b"a.test"));
    srv_raw("P07h GET *", fr(1,5,1,&p)).await;
}
#[tokio::test]
async fn p08_bad_field_kills_conn() {
    let mut good = fr(1,5,3,&get_head());
    let mut p = get_head(); p.extend(lit(b"x", b"a\nb"));
    let mut all = fr(1,5,1,&p); all.append(&mut good);
    srv_raw("P08a newline in value on stream1, good stream3", all).await;
    let mut p = get_head(); p.extend(lit(b"X-Up", b"v"));
    let mut all = fr(1,5,1,&p); all.extend(fr(1,5,3,&get_head()));
    srv_raw("P08c uppercase name on stream1, good stream3", all).await;
    let mut p = get_head(); p.extend(lit(b":foo", b"v"));
    let mut all = fr(1,5,1,&p); all.extend(fr(1,5,3,&get_head()));
    srv_raw("P08d unknown pseudo on stream1, good stream3", all).await;
    let mut p = vec![0x87u8,0x84]; p.extend(lit(b":method", b"G ET"));p.extend(lit(b":authority", b"a.test"));
    let mut all = fr(1,5,1,&p); all.extend(fr(1,5,3,&get_head()));
    srv_raw("P08e bad method on stream1, good stream3", all).await;
    let mut p = get_head(); p.extend(lit(b"connection", b"close"));
    let mut all = fr(1,5,1,&p); all.extend(fr(1,5,3,&get_head()));
    srv_raw("P08f connection hdr on stream1 (control), good stream3", all).await;
}
#[tokio::test]
async fn p09_malformed_across_continuation() {
    let mut block = get_head(); block.extend(lit(b"connection", b"close"));
    let l2 = lit(b"foo", b"bar");
    let mut first = block.clone(); first.extend_from_slice(&l2[..2]);
    let mut all = fr(1,0,1,&first); all.extend(fr(9,4,1,&l2[2..]));
    srv_raw("P09a connection: close, split mid next literal", all).await;

    let mut all = fr(1,0,1,&block); all.extend(fr(9,4,1,&l2)); all.extend(fr(1,5,3,&get_head()));
    srv_raw("P09b connection: close, complete in HEADERS w/o END_HEADERS", all).await;

    // te: gzip split
    let mut block = get_head(); block.extend(lit(b"transfer-encoding", b"chunked"));
    let mut first = block.clone(); first.extend_from_slice(&l2[..1]);
    let mut all = fr(1,0,1,&first); all.extend(fr(9,4,1,&l2[1..]));
    srv_raw("P09c transfer-encoding, split", all).await;

    // repeated pseudo split
    let mut block = get_head(); block.extend_from_slice(&[0x83]);
    let mut first = block.clone(); first.extend_from_slice(&l2[..1]);
    let mut all = fr(1,0,1,&first); all.extend(fr(9,4,1,&l2[1..]));
    srv_raw("P09d duplicate :method, split", all).await;
    // pseudo after regular, regular before split
    let mut block = get_head(); block.extend(l2.clone()); block.extend_from_slice(&[0x83]);
    let mut first = block.clone(); first.extend_from_slice(&l2[..1]);
    let mut all = fr(1,0,1,&first); all.extend(fr(9,4,1,&l2[1..]));
    srv_raw("P09e pseudo after field, split", all).await;
}
#[tokio::test]
async fn p11_trailers_forgotten_stream() {
    let (io, mut srv) = mock::new();
    let m = tokio::spawn(async move {
        srv.assert_server_handshake().await;
        srv.send_frame(frames::headers(1).request("POST", "https://a.test/")).await;
        let f = srv.next().await; println!("P11 got {:?}", f);
        srv.send_frame(frames::data(1, &b"abc"[..])).await;
        let f = srv.next().await; println!("P11 after DATA on forgotten stream {:?}", f);
        srv.send_frame(frames::headers(1).field("foo","bar").eos()).await;
        let f = drain(&mut srv, 300).await;
        println!("P11 frames after trailers on forgotten stream: {:?}", f);
    });
    let mut b = server::Builder::new(); b.max_concurrent_reset_streams(0);
    let mut conn = b.handshake::<_, Bytes>(io).await.unwrap();
    let (r, mut resp) = conn.accept().await.unwrap().unwrap();
    resp.send_reset(Reason::CANCEL);
    drop(r); drop(resp);
    let r = timeout(Duration::from_millis(500), poll_fn(|cx| conn.poll_closed(cx))).await;
    println!("P11 conn {:?}", r);
    let _ = m.await;
}
#[tokio::test]
async fn p12_content_length() {
    let mut p = get_head(); p.extend(lit(b"content-length", b"1")); p.extend(lit(b"content-length", b"100"));
    let mut all = fr(1,4,1,&p); all.extend(fr(0,1,1,b"x"));
    srv_raw("P12a two differing content-length", all).await;
    let mut p = get_head(); p.extend(lit(b"content-length", b""));
    srv_raw("P12b empty content-length no body", fr(1,5,1,&p)).await;
    let mut p = get_head(); p.extend(lit(b"content-length", b"1, 1"));
    let mut all = fr(1,4,1,&p); all.extend(fr(0,1,1,b"x"));
    srv_raw("P12c cl 1, 1", all).await;
    let mut p = get_head(); p.extend(lit(b"content-length", b"99999999999999999999"));
    srv_raw("P12d huge cl", fr(1,5,1,&p)).await;
    let mut p = get_head(); p.extend(lit(b"content-length", b"0000000000000000000000000001"));
    let mut all = fr(1,4,1,&p); all.extend(fr(0,1,1,b"x"));
    srv_raw("P12e long leading zeros", all).await;
}
#[tokio::test]
async fn p15_oversize_trailers() {
    let (io, mut srv) = mock::new();
    let m = tokio::spawn(async move {
        srv.assert_server_handshake().await;
        srv.send_frame(frames::headers(1).request("POST", "https://a.test/")).await;
        srv.send_frame(frames::headers(1).field("keep", "v").field("big", String::from_utf8(vec![b'a'; 500]).unwrap()).eos()).await;
        let f = drain(&mut srv, 300).await;
        println!("P15 frames: {:?}", f);
    });
    let mut b = server::Builder::new(); b.max_header_list_size(300);
    let mut conn = b.handshake::<_, Bytes>(io).await.unwrap();
    let (r, _resp) = conn.accept().await.unwrap().unwrap();
    let mut body = r.into_body();
    tokio::spawn(async move { let _ = timeout(Duration::from_millis(400), poll_fn(|cx| conn.poll_closed(cx))).await; });
    let t = timeout(Duration::from_millis(200), body.trailers()).await;
    println!("P15 trailers observed: {:?}", t);
    let _ = m.await;
}
#[tokio::test]
async fn p17_error_codes() {
    srv_raw("P17a WINDOW_UPDATE 0 on stream", {let mut a = fr(1,4,1,&get_head()); a.extend(fr(8,0,1,&0u32.to_be_bytes())); a}).await;
    srv_raw("P17b SETTINGS initial window too big", fr(4,0,0,&[0,4,0xff,0xff,0xff,0xff])).await;
    srv_raw("P17c PING len 7", fr(6,0,0,&[0;7])).await;
    srv_raw("P17d RST len 3", {let mut a = fr(1,4,1,&get_head()); a.extend(fr(3,0,1,&[0;3])); a}).await;
    srv_raw("P17e SETTINGS len 5", fr(4,0,0,&[0;5])).await;
    srv_raw("P17f WINDOW_UPDATE len 3", fr(8,0,0,&[0;3])).await;
    srv_raw("P17g PRIORITY len 4", fr(2,0,1,&[0;4])).await;
    srv_raw("P17h SETTINGS ack w/ payload", fr(4,1,0,&[0;6])).await;
    srv_raw("P17i HEADERS padded too much", fr(1,0xd,1,&[200,0x82])).await;
    srv_raw("P17j SIZE UPDATE too big", fr(1,5,1,&[0x3f,0xff,0xff,0x7f])).await;
    srv_raw("P17k SIZE UPDATE after field", fr(1,5,1,&[0x82,0x20])).await;
    let mut h = get_head(); let l = lit(b"a",b"b"); h.extend(l);
    srv_raw("P17l SIZE UPDATE at start of CONTINUATION", {let mut a = fr(1,0,1,&h); a.extend(fr(9,4,1,&[0x20])); a}).await;
    srv_raw("P17m RST on idle", fr(3,0,9,&[0,0,0,8])).await;
    srv_raw("P17n client HEADERS on even id", fr(1,5,2,&get_head())).await;
    srv_raw("P17o empty name literal", fr(1,5,1,&[0x82,0x87,0x84,0x00,0x00,0x00])).await;
    srv_raw("P17p empty name literal, no END_HEADERS then CONT", {let mut a = fr(1,0,1,&[0x82,0x87,0x84,0x40,0x00,0x00]); a.extend(fr(9,4,1,&[0x83])); a}).await;
}
