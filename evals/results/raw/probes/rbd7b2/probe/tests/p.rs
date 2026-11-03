use futures::{StreamExt};
use h2_support::prelude::*;
use std::time::Duration;

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
fn idxlit(i: u8, v: &str) -> Vec<u8> {
    let mut o = vec![i, v.len() as u8];
    o.extend_from_slice(v.as_bytes());
    o
}
fn base() -> Vec<u8> {
    let mut b = vec![0x82, 0x87, 0x84];
    b.extend(idxlit(1, "x.com"));
    b
}
async fn drain(srv: &mut mock::Handle, ms: u64, tag: &str) {
    loop {
        match tokio::time::timeout(Duration::from_millis(ms), srv.next()).await {
            Err(_) => { println!("[{tag}] <idle>"); return; }
            Ok(None) => { println!("[{tag}] <eof>"); return; }
            Ok(Some(Ok(f))) => println!("[{tag}] peer got: {:?}", f),
            Ok(Some(Err(e))) => { println!("[{tag}] read err: {:?}", e); return; }
        }
    }
}
/// Feed raw frames to a default h2 server, printing what the app and wire see.
async fn server_raw(tag: &'static str, frames_in: Vec<Vec<u8>>, b: server::Builder) {
    let (io, mut c) = mock::new();
    let cl = async move {
        c.assert_server_handshake().await;
        idle_ms(5).await;
        for f in frames_in { c.send_bytes(&f).await; }
        drain(&mut c, 150, tag).await;
    };
    let s = async move {
        let mut srv = b.handshake::<_, Bytes>(io).await.unwrap();
        while let Some(r) = srv.accept().await {
            match r {
                Ok((req, resp)) => { println!("[{tag}] app accepted {} uri={} headers={:?}", req.method(), req.uri(), req.headers().keys().map(|k| k.as_str().to_string()).collect::<Vec<_>>().join(",")); std::mem::forget(resp); }
                Err(e) => { println!("[{tag}] accept err {:?}", e); break; }
            }
        }
    };
    tokio::time::timeout(Duration::from_secs(20), join(cl, s)).await.ok();
}

#[tokio::test]
async fn p01_push_1xx_then_final() {
    let (io, mut srv) = mock::new();
    let m = async move {
        srv.assert_client_handshake().await;
        srv.recv_frame(frames::headers(1).request("GET", "https://x.com/").eos()).await;
        srv.send_frame(frames::headers(1).response(200)).await;
        srv.send_frame(frames::push_promise(1, 2).request("GET", "https://x.com/p")).await;
        srv.send_frame(frames::headers(2).response(100)).await;
        srv.send_frame(frames::headers(2).response(200)).await;
        idle_ms(100).await;
    };
    let h = async move {
        let (mut client, mut conn) = client::handshake(io).await.unwrap();
        let req = Request::builder().uri("https://x.com/").body(()).unwrap();
        let (resp, _) = client.send_request(req, true).unwrap();
        let _r = conn.drive(resp).await;
        conn.await.ok();
        println!("[p01] client survived");
    };
    join(m, h).await;
}

#[tokio::test]
async fn p02_push_over_max_concurrent() {
    let (io, mut srv) = mock::new();
    let m = async move {
        srv.assert_client_handshake().await;
        srv.recv_frame(frames::headers(1).request("GET", "https://x.com/").eos()).await;
        srv.send_frame(frames::headers(1).response(200)).await;
        srv.send_frame(frames::push_promise(1, 2).request("GET", "https://x.com/a")).await;
        srv.send_frame(frames::push_promise(1, 4).request("GET", "https://x.com/b")).await;
        srv.send_frame(frames::headers(2).response(200)).await;
        srv.send_frame(frames::headers(4).response(200)).await;
        idle_ms(100).await;
    };
    let h = async move {
        let (mut client, mut conn) = client::Builder::new().max_concurrent_streams(1).handshake::<_, Bytes>(io).await.unwrap();
        let req = Request::builder().uri("https://x.com/").body(()).unwrap();
        let (resp, _) = client.send_request(req, true).unwrap();
        let _r = conn.drive(resp).await;
        conn.await.ok();
        println!("[p02] client survived");
    };
    join(m, h).await;
}

#[tokio::test]
async fn p03_padded_data_flow_control() {
    let (io, mut c) = mock::new();
    let cl = async move {
        c.assert_server_handshake().await;
        let mut h = base(); h[0] = 0x83; // POST
        c.send_bytes(&fr(1, 4, 1, &h)).await;
        let mut pl = vec![254u8, b'z']; pl.extend_from_slice(&[0u8; 254]);
        for _ in 0..300 { c.send_bytes(&fr(0, 0x8, 1, &pl)).await; }
        drain(&mut c, 200, "p03").await;
    };
    let s = async move {
        let mut srv = server::handshake(io).await.unwrap();
        let (req, _resp) = srv.accept().await.unwrap().unwrap();
        let mut body = req.into_body();
        let t = async {
            let mut n = 0usize;
            while let Some(Ok(b)) = body.data().await {
                n += b.len();
                body.flow_control().release_capacity(b.len()).unwrap();
                if n == 300 { println!("[p03] app got and released {n} payload bytes of 76800 wire bytes"); break; }
            }
            idle_ms(100).await;
        };
        { let acc = async { tokio::time::timeout(Duration::from_millis(300), srv.accept()).await.ok(); }; join(t, acc).await; }
        tokio::time::timeout(Duration::from_millis(100), srv.accept()).await.ok();
    };
    tokio::time::timeout(Duration::from_secs(10), join(cl, s)).await.ok();
}

#[tokio::test]
async fn p04_malformed_then_continuation() {
    let mut a = base(); a.extend(lit("connection", "close"));
    server_raw("p04", vec![fr(1, 0, 1, &a), fr(9, 4, 1, &lit("a", "b"))], server::Builder::new()).await;
}

#[tokio::test]
async fn p05_malformed_flag_lost_on_needmore() {
    let mut a = base(); a.extend(lit("connection", "close"));
    a.extend_from_slice(&[0, 1, b'a']);
    server_raw("p05", vec![fr(1, 0, 1, &a), fr(9, 5, 1, &[1, b'b'])], server::Builder::new()).await;
}

#[tokio::test]
async fn p05b_needmore_with_duplicate_pseudo() {
    let mut a = base(); a.extend(idxlit(4, "/second")); // duplicate :path
    a.extend_from_slice(&[0, 1, b'a']);
    server_raw("p05b", vec![fr(1, 0, 1, &a), fr(9, 5, 1, &[1, b'b'])], server::Builder::new()).await;
}

#[tokio::test]
async fn p06_missing_status() {
    let (io, mut srv) = mock::new();
    let m = async move {
        srv.assert_client_handshake().await;
        srv.recv_frame(frames::headers(1).request("GET", "https://x.com/").eos()).await;
        srv.send_bytes(&fr(1, 5, 1, &lit("x", "y"))).await;
        idle_ms(50).await;
    };
    let h = async move {
        let (mut client, mut conn) = client::handshake(io).await.unwrap();
        let req = Request::builder().uri("https://x.com/").body(()).unwrap();
        let (resp, _) = client.send_request(req, true).unwrap();
        let r = conn.drive(resp).await;
        println!("[p06] client result: {:?}", r.map(|r| r.status()));
        conn.await.ok();
    };
    join(m, h).await;
}

#[tokio::test]
async fn p07_paths() {
    // no path, no authority
    server_raw("p07a", vec![fr(1, 5, 1, &[0x82, 0x87])], server::Builder::new()).await;
    // path without slash, with authority
    let mut a = vec![0x82, 0x87]; a.extend(idxlit(1, "x.com")); a.extend(idxlit(4, "foo"));
    server_raw("p07b", vec![fr(1, 5, 1, &a)], server::Builder::new()).await;
    let mut a = vec![0x82, 0x87]; a.extend(idxlit(1, "x.com")); a.extend(idxlit(4, "@evil.test/x"));
    server_raw("p07c", vec![fr(1, 5, 1, &a)], server::Builder::new()).await;
    // CONNECT without authority
    let mut a = Vec::new(); a.extend(idxlit(2, "CONNECT"));
    server_raw("p07d", vec![fr(1, 4, 1, &a)], server::Builder::new()).await;
    // pseudo-less request with only method
    server_raw("p07e", vec![fr(1, 5, 1, &[0x82])], server::Builder::new()).await;
    // no method
    server_raw("p07f", vec![fr(1, 5, 1, &[0x87, 0x84])], server::Builder::new()).await;
    // empty authority
    let mut a = vec![0x82, 0x87, 0x84]; a.extend(idxlit(1, ""));
    server_raw("p07g", vec![fr(1, 5, 1, &a)], server::Builder::new()).await;
}

#[tokio::test]
async fn p08_oversize_trailers_truncated() {
    let (io, mut c) = mock::new();
    let cl = async move {
        c.assert_server_handshake().await;
        idle_ms(20).await;
        let mut h = base(); h[0] = 0x83;
        c.send_bytes(&fr(1, 4, 1, &h)).await;
        let big = "v".repeat(120);
        let mut t = lit("grpc-keep", "1"); t.extend(lit("x-big1", &big)); t.extend(lit("x-big2", &big)); t.extend(lit("grpc-status", "13"));
        c.send_bytes(&fr(1, 5, 1, &t)).await;
        drain(&mut c, 150, "p08").await;
    };
    let s = async move {
        let mut srv = server::Builder::new().max_header_list_size(300).handshake::<_, Bytes>(io).await.unwrap();
        let (req, _resp) = srv.accept().await.unwrap().unwrap();
        let mut body = req.into_body();
        let t = async { let r = body.trailers().await; println!("[p08] app trailers: {:?}", r); idle_ms(50).await; };
        { let acc = async { tokio::time::timeout(Duration::from_millis(300), srv.accept()).await.ok(); }; join(t, acc).await; }
    };
    tokio::time::timeout(Duration::from_secs(10), join(cl, s)).await.ok();
}

#[tokio::test]
async fn p09_trailers_with_pseudo() {
    let (io, mut c) = mock::new();
    let cl = async move {
        c.assert_server_handshake().await;
        let mut h = base(); h[0] = 0x83;
        c.send_bytes(&fr(1, 4, 1, &h)).await;
        let mut t = idxlit(4, "/evil"); t.extend(lit("a", "b"));
        c.send_bytes(&fr(1, 5, 1, &t)).await;
        drain(&mut c, 150, "p09").await;
    };
    let s = async move {
        let mut srv = server::handshake(io).await.unwrap();
        let (req, _resp) = srv.accept().await.unwrap().unwrap();
        let mut body = req.into_body();
        let t = async { let r = body.trailers().await; println!("[p09] app trailers: {:?}", r); idle_ms(50).await; };
        { let acc = async { tokio::time::timeout(Duration::from_millis(300), srv.accept()).await.ok(); }; join(t, acc).await; }
    };
    tokio::time::timeout(Duration::from_secs(10), join(cl, s)).await.ok();
}

#[tokio::test]
async fn p10_304_cl_empty_data() {
    let (io, mut srv) = mock::new();
    let m = async move {
        srv.assert_client_handshake().await;
        srv.recv_frame(frames::headers(1).request("GET", "https://x.com/").eos()).await;
        srv.send_frame(frames::headers(1).response(304).field("content-length", 100)).await;
        srv.send_frame(frames::data(1, "").eos()).await;
        drain(&mut srv, 100, "p10").await;
    };
    let h = async move {
        let (mut client, mut conn) = client::handshake(io).await.unwrap();
        let req = Request::builder().uri("https://x.com/").body(()).unwrap();
        let (resp, _) = client.send_request(req, true).unwrap();
        let t = async move {
            let r = resp.await.unwrap();
            println!("[p10] status {}", r.status());
            let mut b = r.into_body();
            println!("[p10] body end: {:?}", b.data().await.map(|x| x.map(|y| y.len())));
        };
        conn.drive(t).await;
        conn.await.ok();
    };
    tokio::time::timeout(Duration::from_secs(10), join(m, h)).await.ok();
}

#[tokio::test]
async fn p11_content_length() {
    let mut a = base(); a.extend(lit("content-length", ""));
    server_raw("p11a-empty", vec![fr(1, 5, 1, &a)], server::Builder::new()).await;
    let mut a = base(); a.extend(lit("content-length", "0")); a.extend(lit("content-length", "100"));
    server_raw("p11b-dup", vec![fr(1, 5, 1, &a)], server::Builder::new()).await;
    let mut a = base(); a.extend(lit("content-length", "+5"));
    server_raw("p11c-plus", vec![fr(1, 5, 1, &a)], server::Builder::new()).await;
}

#[tokio::test]
async fn p12_bad_header_conn_level() {
    let mut a = base(); a.extend(lit("x", "a\nb"));
    server_raw("p12a-newline", vec![fr(1, 4, 1, &base()), fr(1, 4, 3, &a)], server::Builder::new()).await;
    let mut a = base(); a.extend(lit("X-Up", "b"));
    server_raw("p12b-upper", vec![fr(1, 4, 1, &base()), fr(1, 4, 3, &a)], server::Builder::new()).await;
    let mut a = base(); a.extend(lit(":foo", "b"));
    server_raw("p12c-pseudo", vec![fr(1, 4, 1, &base()), fr(1, 4, 3, &a)], server::Builder::new()).await;
    let mut a = vec![0x82, 0x87, 0x84]; a.extend(idxlit(2, "BAD METHOD"));
    server_raw("p12d-method", vec![fr(1, 4, 1, &base()), fr(1, 4, 3, &a)], server::Builder::new()).await;
}

#[tokio::test]
async fn p13_push_promise_empty_fragment() {
    let (io, mut srv) = mock::new();
    let m = async move {
        srv.assert_client_handshake().await;
        srv.recv_frame(frames::headers(1).request("GET", "https://x.com/").eos()).await;
        srv.send_frame(frames::headers(1).response(200)).await;
        srv.send_bytes(&fr(5, 0, 1, &2u32.to_be_bytes())).await;
        srv.send_bytes(&fr(9, 4, 1, &base())).await;
        drain(&mut srv, 100, "p13").await;
    };
    let h = async move {
        let (mut client, mut conn) = client::handshake(io).await.unwrap();
        let req = Request::builder().uri("https://x.com/").body(()).unwrap();
        let (resp, _) = client.send_request(req, true).unwrap();
        let r = conn.drive(resp).await;
        println!("[p13] resp {:?}", r.map(|r| r.status()));
        println!("[p13] conn {:?}", conn.await);
    };
    tokio::time::timeout(Duration::from_secs(5), join(m, h)).await.ok();
}

#[tokio::test]
async fn p14_userinfo() {
    let (io, mut srv) = mock::new();
    let m = async move {
        srv.assert_client_handshake().await;
        match srv.next().await {
            Some(Ok(h2::frame::Frame::Headers(h))) => println!("[p14] {:?}", h.into_parts().0),
            o => println!("[p14] other {:?}", o),
        }
    };
    let h = async move {
        let (mut client, mut conn) = client::handshake(io).await.unwrap();
        let req = Request::builder().uri("https://user:s3cret@x.com/").body(()).unwrap();
        let (resp, _) = client.send_request(req, true).unwrap();
        tokio::time::timeout(Duration::from_millis(50), conn.drive(resp)).await.ok();
    };
    tokio::time::timeout(Duration::from_secs(5), join(m, h)).await.ok();
}

#[tokio::test]
async fn p15_exact_limit() {
    for (tag, n) in [("p15-exact", 205u32), ("p15-plus1", 206u32)] {
        let mut a = base(); a.extend(lit("x", "y"));
        server_raw(tag, vec![fr(1, 5, 1, &a)], { let mut b = server::Builder::new(); b.max_header_list_size(n); b }).await;
    }
}

#[tokio::test]
async fn p17_size_update_mid_block() {
    server_raw("p17a", vec![fr(1, 0, 1, &base()), fr(9, 5, 1, &[0x20])], server::Builder::new()).await;
    server_raw("p17b-same-frame", vec![fr(1, 5, 1, &{ let mut a = base(); a.push(0x20); a })], server::Builder::new()).await;
    server_raw("p17c-index0", vec![fr(1, 5, 1, &[0x80])], server::Builder::new()).await;
    server_raw("p17d-bad-huff", vec![fr(1, 5, 1, &[0x40, 0x81, 0xff, 0x00])], server::Builder::new()).await;
}

#[tokio::test]
async fn p18_codes() {
    let b = || server::Builder::new();
    server_raw("p18-ping7", vec![fr(6, 0, 0, &[0; 7])], b()).await;
    server_raw("p18-ack-payload", vec![fr(4, 1, 0, &[0; 6])], b()).await;
    server_raw("p18-settings5", vec![fr(4, 0, 0, &[0; 5])], b()).await;
    server_raw("p18-rst3", vec![fr(3, 0, 1, &[0; 3])], b()).await;
    server_raw("p18-wu3", vec![fr(8, 0, 0, &[0; 3])], b()).await;
    server_raw("p18-prio4", vec![fr(2, 0, 1, &[0; 4])], b()).await;
    server_raw("p18-wu0-stream", vec![fr(1, 4, 1, &base()), fr(8, 0, 1, &[0; 4])], b()).await;
    server_raw("p18-window-big", vec![fr(4, 0, 0, &[0, 4, 0x80, 0, 0, 0])], b()).await;
    server_raw("p18-goaway-stream1", vec![fr(7, 0, 1, &[0; 8])], b()).await;
    server_raw("p18-pad-too-much", vec![fr(1, 0xc, 1, &[200, 0x82])], b()).await;
}

#[tokio::test]
async fn p19_push_dropped_one() { push_drop(1, "p19-1").await; }
#[tokio::test]
async fn p19_push_dropped_many() { push_drop(20, "p19-20").await; }

async fn push_drop(n: u32, tag: &'static str) {
    let (io, mut c) = mock::new();
    let cl = async move {
        c.assert_server_handshake().await;
        c.send_bytes(&fr(1, 5, 1, &base())).await;
        drain(&mut c, 200, tag).await;
    };
    let s = async move {
        let mut srv = server::handshake(io).await.unwrap();
        let (_req, mut resp) = srv.accept().await.unwrap().unwrap();
        let mut st = resp.send_response(Response::new(()), false).unwrap();
        for _ in 0..n {
            let p = resp.push_request(Request::builder().uri("https://x.com/p").body(()).unwrap()).unwrap();
            drop(p);
        }
        st.send_data(Bytes::new(), true).unwrap();
        tokio::time::timeout(Duration::from_millis(150), srv.accept()).await.ok();
        println!("[{tag}] server survived");
    };
    tokio::time::timeout(Duration::from_secs(5), join(cl, s)).await.ok();
}

#[tokio::test]
async fn p20_many_unique_headers() {
    for (tag, count) in [("p20-20k", 20000u32), ("p20-40k", 40000u32)] {
        let mut blk = base();
        for i in 0..count { blk.extend(lit(&format!("h{:05}", i), "")); }
        let mut fs = Vec::new();
        let mut it = blk.chunks(16384);
        let first = it.next().unwrap();
        let chunks: Vec<_> = it.collect();
        fs.push(fr(1, 1, 1, first));
        for (i, c) in chunks.iter().enumerate() {
            let flags = if i + 1 == chunks.len() { 4 } else { 0 };
            fs.push(fr(9, flags, 1, c));
        }
        println!("[{tag}] wire bytes {}, frames {}", blk.len(), fs.len());
        server_raw(tag, fs, server::Builder::new()).await;
    }
}

#[tokio::test]
async fn p21_goaway_window_leak() {
    let (io, mut c) = mock::new();
    let cl = async move {
        c.assert_server_handshake().await;
        let mut h = base(); h[0] = 0x83;
        c.send_bytes(&fr(1, 4, 1, &h)).await;
        // wait for graceful GOAWAY + PING
        idle_ms(50).await;
        c.send_bytes(&fr(1, 4, 3, &h)).await; // raced new stream
        c.send_bytes(&fr(6, 1, 0, &[0x0b, 0x7b, 0xa2, 0xf0, 0x8b, 0x9b, 0xfe, 0x54])).await; // shutdown ping ack
        idle_ms(20).await;
        let d = vec![b'z'; 16384];
        for _ in 0..3 { c.send_bytes(&fr(0, 0, 3, &d)).await; }
        drain(&mut c, 300, "p21").await;
    };
    let s = async move {
        let mut srv = server::handshake(io).await.unwrap();
        let (_req, _resp) = srv.accept().await.unwrap().unwrap();
        srv.graceful_shutdown();
        tokio::time::timeout(Duration::from_millis(500), srv.accept()).await.ok();
    };
    tokio::time::timeout(Duration::from_secs(5), join(cl, s)).await.ok();
}
