#![allow(dead_code, unused)]
use futures::{StreamExt, TryStreamExt, future::poll_fn};
use h2_support::prelude::*;
use std::time::Duration;

// ---- raw helpers ----
fn raw(kind: u8, flags: u8, id: u32, payload: &[u8]) -> Vec<u8> {
    let l = payload.len();
    let mut v = vec![(l >> 16) as u8, (l >> 8) as u8, l as u8, kind, flags];
    v.extend_from_slice(&id.to_be_bytes());
    v.extend_from_slice(payload);
    v
}
// literal without indexing, new name, no huffman
fn lit(name: &str, val: &str) -> Vec<u8> {
    let mut v = vec![0u8];
    enc_str(name.as_bytes(), &mut v);
    enc_str(val.as_bytes(), &mut v);
    v
}
fn lit_idx(name: &str, val: &str) -> Vec<u8> {
    let mut v = vec![0x40u8];
    enc_str(name.as_bytes(), &mut v);
    enc_str(val.as_bytes(), &mut v);
    v
}
fn enc_str(s: &[u8], v: &mut Vec<u8>) {
    if s.len() < 127 { v.push(s.len() as u8); } else {
        v.push(127);
        let mut n = s.len() - 127;
        while n >= 128 { v.push((n % 128) as u8 | 0x80); n /= 128; }
        v.push(n as u8);
    }
    v.extend_from_slice(s);
}
// :method GET, :scheme https, :path /, :authority a
fn req_pseudo() -> Vec<u8> {
    let mut v = vec![0x82, 0x87, 0x84];
    v.extend(lit(":authority", "a"));
    v
}
const END_STREAM: u8 = 1; const END_HEADERS: u8 = 4; const PADDED: u8 = 8; const PRIORITY: u8 = 0x20;

/// Print every frame arriving for `ms` milliseconds
async fn drain(srv: &mut h2_support::mock::Handle, ms: u64, tag: &str) -> Vec<String> {
    let mut out = vec![];
    loop {
        match tokio::time::timeout(Duration::from_millis(ms), srv.next()).await {
            Ok(Some(Ok(f))) => { let s = format!("{:?}", f); println!("[{}] {}", tag, s); out.push(s); }
            Ok(Some(Err(e))) => { println!("[{}] ERR {:?}", tag, e); out.push(format!("ERR {:?}", e)); break; }
            Ok(None) => { println!("[{}] EOF", tag); out.push("EOF".into()); break; }
            Err(_) => break,
        }
    }
    out
}

fn get() -> Request<()> {
    Request::builder().method(Method::GET).uri("https://a/").body(()).unwrap()
}
fn post() -> Request<()> {
    Request::builder().method(Method::POST).uri("https://a/").body(()).unwrap()
}

/// server loop accepting requests and printing them
async fn server_print(io: h2_support::mock::Mock) {
    let mut srv = server::handshake(io).await.unwrap();
    while let Some(r) = srv.next().await {
        match r {
            Ok((req, mut resp)) => {
                println!("[srv] ACCEPT {:?} {:?}", req.method(), req.headers());
                resp.send_response(Response::builder().status(200).body(()).unwrap(), true).ok();
            }
            Err(e) => { println!("[srv] conn err {:?}", e); return; }
        }
    }
    println!("[srv] conn closed ok");
}

// T1
#[tokio::test]
async fn t01_padded_data_not_counted() {
    let (io, mut c) = mock::new();
    let peer = async move {
        c.assert_server_handshake().await;
        let mut h = req_pseudo();
        c.send_bytes(&raw(1, END_HEADERS, 1, &h)).await;
        // 255 zero-payload frames with 255 bytes of padding = 65,280 flow controlled bytes
        let mut p = vec![255u8]; p.extend(vec![0u8; 255]);
        for _ in 0..300 { c.send_bytes(&raw(0, PADDED, 1, &p)).await; }
        let mut p2 = vec![200u8]; p2.extend(vec![b'x'; 16000]); p2.extend(vec![0u8; 200]);
        c.send_bytes(&raw(0, PADDED, 1, &p2)).await;
        c.send_bytes(&raw(0, PADDED | END_STREAM, 1, &p2)).await;
        tokio::time::sleep(Duration::from_millis(300)).await;
        drain(&mut c, 200, "client").await;
    };
    let s = async move {
        let mut srv = server::handshake(io).await.unwrap();
        let (req, mut resp) = srv.next().await.unwrap().unwrap();
        let mut body = req.into_body();
        let fc = body.flow_control().clone();
        let drive = async {
            let mut total = 0;
            while let Some(d) = body.data().await { let d = d.unwrap(); total += d.len(); body.flow_control().release_capacity(d.len()).unwrap(); }
            println!("[srv] body bytes {}", total);
            resp.send_response(Response::builder().status(200).body(()).unwrap(), true).unwrap();
        };
        let srv2 = async { let _ = srv.next().await; };
        tokio::select!{ _ = join(drive, srv2) => {}, _ = tokio::time::sleep(Duration::from_millis(800)) => {} };
    };
    join(peer, s).await;
}

// T2
#[tokio::test]
async fn t02_self_dep_hpack_desync() {
    let (io, mut c) = mock::new();
    let peer = async move {
        c.assert_server_handshake().await;
        let mut p = vec![0, 0, 0, 1, 16]; // dep on self
        p.extend(req_pseudo());
        p.extend(lit_idx("x-a", "b"));
        c.send_bytes(&raw(1, END_HEADERS | END_STREAM | PRIORITY, 1, &p)).await;
        // stream 3 refers to index 62 which peer inserted in stream 1's block
        let mut h = req_pseudo(); h.push(0x80 | 62);
        c.send_bytes(&raw(1, END_HEADERS | END_STREAM, 3, &h)).await;
        drain(&mut c, 300, "client").await;
    };
    join(peer, server_print(io)).await;
}

// T3
#[tokio::test]
async fn t03_malformed_forgotten_on_needmore() {
    let (io, mut c) = mock::new();
    let peer = async move {
        c.assert_server_handshake().await;
        let mut h = req_pseudo();
        h.extend(lit("te", "gzip"));
        h.extend(lit("connection", "close"));
        let l = lit("x-after", "value");
        let (l1, l2) = l.split_at(3);
        h.extend_from_slice(l1);
        c.send_bytes(&raw(1, END_STREAM, 1, &h)).await;
        c.send_bytes(&raw(9, END_HEADERS, 1, l2)).await;
        drain(&mut c, 300, "client").await;
    };
    join(peer, server_print(io)).await;
}

// T4
#[tokio::test]
async fn t04_malformed_then_continuation() {
    let (io, mut c) = mock::new();
    let peer = async move {
        c.assert_server_handshake().await;
        let mut h = req_pseudo();
        h.extend(lit("connection", "close"));
        c.send_bytes(&raw(1, END_STREAM, 1, &h)).await;
        c.send_bytes(&raw(9, END_HEADERS, 1, &lit("x-after", "value"))).await;
        drain(&mut c, 300, "client").await;
    };
    join(peer, server_print(io)).await;
}

// T5
#[tokio::test]
async fn t05_many_distinct_headers() {
    let (io, mut c) = mock::new();
    let peer = async move {
        c.assert_server_handshake().await;
        let mut block = req_pseudo();
        for i in 0..40000u32 { block.extend(lit(&format!("x{:05}", i), "")); }
        println!("block bytes {}", block.len());
        let chunks: Vec<&[u8]> = block.chunks(16384).collect();
        println!("frames {}", chunks.len());
        for (i, ch) in chunks.iter().enumerate() {
            let last = i == chunks.len() - 1;
            let flags = if last { END_HEADERS } else { 0 };
            if i == 0 { c.send_bytes(&raw(1, flags | END_STREAM, 1, ch)).await; }
            else { c.send_bytes(&raw(9, flags, 1, ch)).await; }
        }
        drain(&mut c, 500, "client").await;
    };
    let h = tokio::spawn(server_print(io));
    peer.await;
    println!("server task result: {:?}", h.await.map_err(|e| e.is_panic()));
}

// T6
#[tokio::test]
async fn t06_1xx_on_pushed_stream() {
    let (io, mut srv) = mock::new();
    let mock = async move {
        srv.assert_client_handshake().await;
        srv.recv_frame(frames::headers(1).request("GET", "https://a/").eos()).await;
        srv.send_frame(frames::push_promise(1, 2).request("GET", "https://a/x")).await;
        srv.send_frame(frames::headers(2).response(103)).await;
        srv.send_frame(frames::headers(2).response(200).eos()).await;
        srv.send_frame(frames::headers(1).response(200).eos()).await;
        drain(&mut srv, 300, "srv").await;
    };
    let cl = tokio::spawn(async move {
        let (mut client, h2) = client::handshake(io).await.unwrap();
        tokio::spawn(async move { let r = h2.await; println!("conn result {:?}", r); });
        let (mut resp, _) = client.send_request(get(), true).unwrap();
        let mut pushed = resp.push_promises();
        let r = resp.await; println!("resp {:?}", r.map(|r| r.status()));
        while let Some(p) = pushed.next().await { println!("push {:?}", p.map(|p| p.into_parts().0.uri().clone())); }
    });
    mock.await;
    println!("client task: {:?}", cl.await.map_err(|e| e.is_panic()));
}

// T7
#[tokio::test]
async fn t07_pushed_headers_exceed_local_max_concurrent() {
    let (io, mut srv) = mock::new();
    let mock = async move {
        srv.assert_client_handshake_with_settings(frames::settings()).await;
        srv.recv_frame(frames::headers(1).request("GET", "https://a/").eos()).await;
        srv.send_frame(frames::push_promise(1, 2).request("GET", "https://a/x")).await;
        srv.send_frame(frames::push_promise(1, 4).request("GET", "https://a/y")).await;
        srv.send_frame(frames::headers(2).response(200)).await;
        srv.send_frame(frames::headers(4).response(200)).await;
        drain(&mut srv, 300, "srv").await;
    };
    let cl = tokio::spawn(async move {
        let (mut client, h2) = client::Builder::new().max_concurrent_streams(1).handshake::<_, Bytes>(io).await.unwrap();
        let conn = tokio::spawn(async move { let r = h2.await; println!("conn result {:?}", r); });
        let (resp, _) = client.send_request(get(), true).unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;
        println!("conn task finished? {:?}", conn.is_finished());
        println!("conn join {:?}", conn.await.map_err(|e| e.is_panic()));
        drop(resp);
    });
    mock.await;
    println!("client task: {:?}", cl.await.map_err(|e| e.is_panic()));
}

// T8
#[tokio::test]
async fn t08_push_promise_after_local_reset() {
    let (io, mut srv) = mock::new();
    let mock = async move {
        srv.assert_client_handshake().await;
        srv.recv_frame(frames::headers(1).request("GET", "https://a/").eos()).await;
        srv.recv_frame(frames::reset(1).cancel()).await;
        srv.send_frame(frames::push_promise(1, 2).request("GET", "https://a/x")).await;
        srv.send_frame(frames::headers(2).response(200).eos()).await;
        drain(&mut srv, 300, "srv").await;
    };
    let cl = tokio::spawn(async move {
        let (mut client, h2) = client::handshake(io).await.unwrap();
        let conn = tokio::spawn(async move { let r = h2.await; println!("conn result {:?}", r); });
        let (resp, _) = client.send_request(get(), true).unwrap();
        drop(resp);
        // keep client alive
        tokio::time::sleep(Duration::from_millis(500)).await;
        println!("conn join {:?}", conn.is_finished());
        drop(client);
        println!("conn join {:?}", conn.await.map_err(|e| e.is_panic()));
    });
    mock.await;
    println!("client task: {:?}", cl.await.map_err(|e| e.is_panic()));
}

// T9
#[tokio::test]
async fn t09_push_reset_counter_never_decremented() {
    let (io, mut srv) = mock::new();
    let mock = async move {
        srv.assert_client_handshake().await;
        srv.recv_frame(frames::headers(1).request("GET", "https://a/").eos()).await;
        srv.send_frame(frames::headers(1).response(200)).await;
        for i in 0..25u32 {
            let id = 2 + i * 2;
            srv.send_frame(frames::push_promise(1, id).request("GET", "https://a/x")).await;
            srv.send_frame(frames::reset(id).cancel()).await;
            tokio::time::sleep(Duration::from_millis(5)).await; // let client accept each
        }
        drain(&mut srv, 300, "srv").await;
    };
    let cl = async move {
        let (mut client, h2) = client::handshake(io).await.unwrap();
        let conn = tokio::spawn(async move { let r = h2.await; println!("conn result {:?}", r); });
        let (mut resp, _) = client.send_request(get(), true).unwrap();
        let mut pushed = resp.push_promises();
        let mut n = 0;
        while let Some(p) = pushed.next().await {
            match p { Ok(p) => { n += 1; drop(p); } Err(e) => { println!("push err after {} accepted: {:?}", n, e); break; } }
        }
        println!("accepted {} pushes", n);
        drop(resp);
    };
    tokio::select!{ _ = join(mock, cl) => {}, _ = tokio::time::sleep(Duration::from_secs(3)) => {println!("timeout");} };
}

// T10
#[tokio::test]
async fn t10_trailers_oversize_and_pseudo() {
    let (io, mut c) = mock::new();
    let peer = async move {
        c.assert_server_handshake_with_settings(frames::settings().max_header_list_size(200)).await;
        c.send_bytes(&raw(1, END_HEADERS, 1, &req_pseudo())).await;
        let mut t = vec![];
        for i in 0..20 { t.extend(lit(&format!("x-tr{:02}", i), "v")); }
        c.send_bytes(&raw(1, END_HEADERS | END_STREAM, 1, &t)).await;
        c.send_bytes(&raw(1, END_HEADERS, 3, &req_pseudo())).await;
        let mut t = lit(":path", "/evil"); t.extend(lit("x-ok", "1"));
        c.send_bytes(&raw(1, END_HEADERS | END_STREAM, 3, &t)).await;
        drain(&mut c, 300, "client").await;
    };
    let s = async move {
        let mut srv = server::Builder::new().max_header_list_size(200).handshake::<_, Bytes>(io).await.unwrap();
        let mut hs = vec![];
        loop {
            match tokio::time::timeout(Duration::from_millis(200), srv.next()).await {
                Ok(Some(Ok((req, resp)))) => { hs.push((req, resp)); }
                _ => break,
            }
        }
        for (req, _r) in hs.iter_mut() {
            let id = _r.stream_id();
            let b = req.body_mut();
            println!("stream {:?} data {:?}", id, poll_fn(|cx| std::task::Poll::Ready(())).await);
            let d = tokio::time::timeout(Duration::from_millis(50), b.data()).await;
            println!(" data->{:?}", d.map(|x| x.map(|y| y.map(|z| z.len()))));
            let t = tokio::time::timeout(Duration::from_millis(50), b.trailers()).await;
            println!(" trailers->{:?}", t.map(|x| x.map(|o| o.map(|m| m.keys().map(|k| k.to_string()).collect::<Vec<_>>()))));
        }
    };
    join(peer, s).await;
}

// T11
#[tokio::test]
async fn t11_content_length_variants() {
    let (io, mut c) = mock::new();
    let peer = async move {
        c.assert_server_handshake().await;
        let mut h = req_pseudo(); h.extend(lit("content-length", "5")); h.extend(lit("content-length", "100"));
        c.send_bytes(&raw(1, END_HEADERS, 1, &h)).await;
        c.send_bytes(&raw(0, END_STREAM, 1, b"hello")).await;
        let mut h = req_pseudo(); h.extend(lit("content-length", ""));
        c.send_bytes(&raw(1, END_HEADERS | END_STREAM, 3, &h)).await;
        drain(&mut c, 300, "client").await;
    };
    join(peer, server_print(io)).await;
}

// T12
#[tokio::test]
async fn t12_1xx_content_length_sticks() {
    let (io, mut srv) = mock::new();
    let mock = async move {
        srv.assert_client_handshake().await;
        srv.recv_frame(frames::headers(1).request("GET", "https://a/").eos()).await;
        srv.send_frame(frames::headers(1).response(103).field("content-length", "0")).await;
        srv.send_frame(frames::headers(1).response(200)).await;
        srv.send_frame(frames::data(1, "hello").eos()).await;
        drain(&mut srv, 300, "srv").await;
    };
    let cl = async move {
        let (mut client, h2) = client::handshake(io).await.unwrap();
        let conn = tokio::spawn(async move { let r = h2.await; println!("conn result {:?}", r); });
        let (resp, _) = client.send_request(get(), true).unwrap();
        let r = resp.await.unwrap();
        println!("status {:?}", r.status());
        let b = util::concat(r.into_body()).await;
        println!("body {:?}", b);
        drop(client);
        let _ = conn.await;
    };
    join(mock, cl).await;
}

// T12b 304 with content-length without END_STREAM on HEADERS, ending with empty DATA
#[tokio::test]
async fn t12b_304_content_length() {
    let (io, mut srv) = mock::new();
    let mock = async move {
        srv.assert_client_handshake().await;
        srv.recv_frame(frames::headers(1).request("GET", "https://a/").eos()).await;
        srv.send_frame(frames::headers(1).response(304).field("content-length", "100")).await;
        srv.send_frame(frames::data(1, "").eos()).await;
        drain(&mut srv, 300, "srv").await;
    };
    let cl = async move {
        let (mut client, h2) = client::handshake(io).await.unwrap();
        let conn = tokio::spawn(async move { let r = h2.await; println!("conn result {:?}", r); });
        let (resp, _) = client.send_request(get(), true).unwrap();
        let r = resp.await.unwrap();
        println!("status {:?}", r.status());
        let b = util::concat(r.into_body()).await;
        println!("body {:?}", b);
        drop(client);
        let _ = conn.await;
    };
    join(mock, cl).await;
}

// T13
#[tokio::test]
async fn t13_reserve_capacity_on_pending_open_starves() {
    let (io, mut srv) = mock::new();
    let mock = async move {
        srv.assert_client_handshake_with_settings(frames::settings().max_concurrent_streams(1)).await;
        drain(&mut srv, 500, "srv").await;
    };
    let cl = async move {
        let (client, h2) = client::handshake(io).await.unwrap();
        let conn = tokio::spawn(async move { let r = h2.await; println!("conn result {:?}", r); });
        let mut c1 = client.ready().await.unwrap();
        let (_r1, mut s1) = c1.send_request(post(), false).unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        let mut c2 = c1.clone().ready().await.unwrap();
        let (_r3, mut s3) = c2.send_request(post(), false).unwrap();
        println!("s3 id pending open");
        s3.reserve_capacity(65535);
        tokio::time::sleep(Duration::from_millis(50)).await;
        println!("s3 cap = {} (stream not open yet)", s3.capacity());
        s1.send_data(Bytes::from(vec![0u8; 1000]), true).unwrap();
        s1.reserve_capacity(1000);
        tokio::time::sleep(Duration::from_millis(300)).await;
        println!("s1 cap = {}", s1.capacity());
        std::mem::forget((_r1, _r3, s1, s3, c1, c2));
        conn.abort();
    };
    join(mock, cl).await;
}

// T14
// T15
#[tokio::test]
async fn t15_recv_stream_drop_holds_conn_window() {
    let (io, mut c) = mock::new();
    let peer = async move {
        c.assert_server_handshake().await;
        c.send_bytes(&raw(1, END_HEADERS, 1, &req_pseudo())).await;
        for _ in 0..3 { c.send_bytes(&raw(0, 0, 1, &vec![1u8; 16384])).await; }
        tokio::time::sleep(Duration::from_millis(200)).await;
        drain(&mut c, 200, "client").await;
    };
    let s = async move {
        let mut srv = server::handshake(io).await.unwrap();
        let (req, mut resp) = srv.next().await.unwrap().unwrap();
        tokio::select!{ _ = srv.next() => {}, _ = tokio::time::sleep(Duration::from_millis(50)) => {} };
        let st = resp.send_response(Response::builder().status(200).body(()).unwrap(), false).unwrap();
        drop(req);
        println!("[srv] dropped RecvStream, holding SendStream");
        tokio::select!{ _ = srv.next() => {}, _ = tokio::time::sleep(Duration::from_millis(500)) => {} };
        drop(st);
    };
    join(peer, s).await;
}

// T15b: parent drop with unaccepted push that has data -> connection window leak
// T16
#[tokio::test]
async fn t16_max_header_list_boundary() {
    let (io, mut c) = mock::new();
    let peer = async move {
        c.assert_server_handshake_with_settings(frames::settings().max_header_list_size(200)).await;
        // :method(7+3+32=42 by real name) ... compute: h2 counts method: 6+1+3+32=42, scheme 6+1+5+32=44, path 4+1+1+32=38, authority 9+1+1+32=43 => 167; field "abc"+"" => 3+0+32=35 => 202 ; use a field with total 33
        let mut h = req_pseudo(); h.extend(lit("x", "")); // 1+32 =33 => 200 exactly
        c.send_bytes(&raw(1, END_HEADERS | END_STREAM, 1, &h)).await;
        drain(&mut c, 200, "client").await;
    };
    let s = async move {
        let mut srv = server::Builder::new().max_header_list_size(200).handshake::<_, Bytes>(io).await.unwrap();
        tokio::select!{ r = srv.next() => println!("{:?}", r.map(|x| x.map(|y| y.0.headers().clone()))), _ = tokio::time::sleep(Duration::from_millis(300)) => println!("no request accepted") };
    };
    join(peer, s).await;
}

// T17
#[tokio::test]
async fn t17_uppercase_header_kills_conn() {
    let (io, mut c) = mock::new();
    let peer = async move {
        c.assert_server_handshake().await;
        let mut h = req_pseudo(); h.extend(lit("X-Up", "1"));
        c.send_bytes(&raw(1, END_HEADERS | END_STREAM, 1, &h)).await;
        drain(&mut c, 200, "client").await;
    };
    join(peer, server_print(io)).await;
}

// T17b: header value with newline; missing :method handled per stream
#[tokio::test]
async fn t17b_bad_value_kills_conn() {
    let (io, mut c) = mock::new();
    let peer = async move {
        c.assert_server_handshake().await;
        let mut h = req_pseudo(); h.extend(lit("x-up", "a\r\nb"));
        c.send_bytes(&raw(1, END_HEADERS | END_STREAM, 1, &h)).await;
        drain(&mut c, 200, "client").await;
    };
    join(peer, server_print(io)).await;
}

// T18
#[tokio::test]
async fn t18_empty_name_in_non_final_frame() {
    let (io, mut c) = mock::new();
    let peer = async move {
        c.assert_server_handshake().await;
        let mut h = req_pseudo(); h.extend(lit("", "v"));
        c.send_bytes(&raw(1, END_STREAM, 1, &h)).await;
        c.send_bytes(&raw(9, END_HEADERS, 1, &lit("x-after", "value"))).await;
        drain(&mut c, 200, "client").await;
    };
    join(peer, server_print(io)).await;
}

// T19
#[tokio::test]
async fn t19_size_update_mid_block() {
    let (io, mut c) = mock::new();
    let peer = async move {
        c.assert_server_handshake().await;
        let h = req_pseudo();
        c.send_bytes(&raw(1, END_STREAM, 1, &h)).await;
        let mut k = vec![0x20]; k.extend(lit("x-after", "value"));
        c.send_bytes(&raw(9, END_HEADERS, 1, &k)).await;
        drain(&mut c, 200, "client").await;
    };
    join(peer, server_print(io)).await;
}

// T20
#[tokio::test]
async fn t20_push_promise_empty_fragment() {
    let (io, mut srv) = mock::new();
    let mock = async move {
        srv.assert_client_handshake().await;
        srv.recv_frame(frames::headers(1).request("GET", "https://a/").eos()).await;
        // PUSH_PROMISE, 4 byte payload then CONTINUATION carrying the block
        let mut pl = vec![];
        srv.send_bytes(&raw(5, 0, 1, &[0, 0, 0, 2])).await;
        pl.extend(req_pseudo());
        srv.send_bytes(&raw(9, END_HEADERS, 1, &pl)).await;
        srv.send_frame(frames::headers(1).response(200).eos()).await;
        drain(&mut srv, 200, "srv").await;
    };
    let cl = async move {
        let (mut client, h2) = client::handshake(io).await.unwrap();
        let conn = tokio::spawn(async move { let r = h2.await; println!("conn result {:?}", r); });
        let (resp, _) = client.send_request(get(), true).unwrap();
        println!("{:?}", resp.await.map(|r| r.status()));
    };
    join(mock, cl).await;
}

// ---------------- batch 2 ----------------
#[tokio::test]
async fn t06b_panic_hangs_user_futures() {
    let (io, mut srv) = mock::new();
    let mock = async move {
        srv.assert_client_handshake().await;
        srv.recv_frame(frames::headers(1).request("GET", "https://a/").eos()).await;
        srv.send_frame(frames::push_promise(1, 2).request("GET", "https://a/x")).await;
        srv.send_frame(frames::headers(2).response(103)).await;
        srv.send_frame(frames::headers(2).response(200).eos()).await;
        drain(&mut srv, 200, "srv").await;
    };
    let cl = async move {
        let (mut client, h2) = client::handshake(io).await.unwrap();
        let conn = tokio::spawn(async move { let r = h2.await; println!("conn result {:?}", r); });
        let (resp, _) = client.send_request(get(), true).unwrap();
        let r = tokio::time::timeout(Duration::from_millis(500), resp).await;
        println!("user future after conn panic: {:?}", r.map(|x| x.map(|y| y.status())));
        println!("conn task {:?}", conn.await.map_err(|e| e.is_panic()));
        std::mem::forget(client);
    };
    join(mock, cl).await;
}

#[tokio::test]
async fn t08b_push_after_user_reset() {
    let (io, mut srv) = mock::new();
    let mock = async move {
        srv.assert_client_handshake().await;
        srv.recv_frame(frames::headers(1).request("POST", "https://a/")).await;
        srv.recv_frame(frames::reset(1).cancel()).await;
        srv.send_frame(frames::push_promise(1, 2).request("GET", "https://a/x")).await;
        drain(&mut srv, 300, "srv").await;
    };
    let cl = async move {
        let (mut client, h2) = client::handshake(io).await.unwrap();
        let conn = tokio::spawn(async move { let r = h2.await; println!("conn result {:?}", r); });
        let (resp, mut st) = client.send_request(post(), false).unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        st.send_reset(Reason::CANCEL);
        tokio::time::sleep(Duration::from_millis(350)).await;
        println!("conn {:?}", conn.is_finished());
        std::mem::forget((resp, st, client));
    };
    join(mock, cl).await;
}

#[tokio::test]
async fn t08c_push_after_remote_reset() {
    let (io, mut srv) = mock::new();
    let mock = async move {
        srv.assert_client_handshake().await;
        srv.recv_frame(frames::headers(1).request("POST", "https://a/")).await;
        srv.send_frame(frames::reset(1).cancel()).await;
        srv.send_frame(frames::push_promise(1, 2).request("GET", "https://a/x")).await;
        drain(&mut srv, 300, "srv").await;
    };
    let cl = async move {
        let (mut client, h2) = client::handshake(io).await.unwrap();
        let conn = tokio::spawn(async move { let r = h2.await; println!("conn result {:?}", r); });
        let (resp, st) = client.send_request(post(), false).unwrap();
        tokio::time::sleep(Duration::from_millis(400)).await;
        println!("conn finished {:?}", conn.is_finished());
        std::mem::forget((resp, st, client));
    };
    join(mock, cl).await;
}

#[tokio::test]
async fn t14_stream_leak_after_capacity() {
    let (io, mut srv) = mock::new();
    let mock = async move {
        srv.assert_client_handshake().await;
        tokio::time::sleep(Duration::from_millis(80)).await;
        drain(&mut srv, 20, "srv").await;
        srv.send_frame(frames::reset(3).cancel()).await;
        srv.send_frame(frames::reset(5).cancel()).await;
        tokio::time::sleep(Duration::from_millis(200)).await;
        srv.send_frame(frames::window_update(0, 100000)).await;
        drain(&mut srv, 400, "srv").await;
    };
    let cl = async move {
        let (mut client, mut h2) = client::handshake(io).await.unwrap();
        let (_r1, mut s1) = client.send_request(post(), false).unwrap();
        s1.send_data(Bytes::from(vec![0u8; 65535]), false).unwrap();
        h2.drive(tokio::time::sleep(Duration::from_millis(30))).await;
        let (r3, mut s3) = client.send_request(post(), false).unwrap();
        let (r5, mut s5) = client.send_request(post(), false).unwrap();
        s3.reserve_capacity(100);
        s5.reserve_capacity(100);
        h2.drive(tokio::time::sleep(Duration::from_millis(100))).await;
        println!("s3 cap {}", s3.capacity());
        h2.drive(tokio::time::sleep(Duration::from_millis(50))).await;
        drop((r3, s3, r5, s5));
        h2.drive(tokio::time::sleep(Duration::from_millis(50))).await;
        println!("wired before window update {} active {}", client.num_wired_streams(), client.num_active_streams());
        h2.drive(tokio::time::sleep(Duration::from_millis(250))).await;
        println!("wired after window update {} active={}", client.num_wired_streams(), client.num_active_streams());
        std::mem::forget((client, _r1, s1, h2));
    };
    join(mock, cl).await;
}

#[tokio::test]
async fn t15b_push_data_window_leak_on_parent_drop() {
    let (io, mut srv) = mock::new();
    let mock = async move {
        srv.assert_client_handshake().await;
        srv.recv_frame(frames::headers(1).request("GET", "https://a/").eos()).await;
        srv.send_frame(frames::headers(1).response(200)).await;
        srv.send_frame(frames::push_promise(1, 2).request("GET", "https://a/x")).await;
        srv.send_frame(frames::headers(2).response(200)).await;
        for _ in 0..3 { srv.send_frame(frames::data(2, vec![0u8; 16384])).await; }
        drain(&mut srv, 500, "srv").await;
    };
    let cl = async move {
        let (mut client, mut h2) = client::handshake(io).await.unwrap();
        let (resp, _) = client.send_request(get(), true).unwrap();
        h2.drive(tokio::time::sleep(Duration::from_millis(100))).await;
        drop(resp);
        h2.drive(tokio::time::sleep(Duration::from_millis(250))).await;
        println!("wired {}", client.num_wired_streams());
        std::mem::forget((client, h2));
    };
    join(mock, cl).await;
}

// control: same with a normal stream => WINDOW_UPDATE appears
#[tokio::test]
async fn t15c_control_normal_stream_releases() {
    let (io, mut srv) = mock::new();
    let mock = async move {
        srv.assert_client_handshake().await;
        srv.recv_frame(frames::headers(1).request("GET", "https://a/").eos()).await;
        srv.send_frame(frames::headers(1).response(200)).await;
        for _ in 0..3 { srv.send_frame(frames::data(1, vec![0u8; 16384])).await; }
        drain(&mut srv, 500, "srv").await;
    };
    let cl = async move {
        let (mut client, mut h2) = client::handshake(io).await.unwrap();
        let (resp, _) = client.send_request(get(), true).unwrap();
        let r = h2.drive(resp).await;
        h2.drive(tokio::time::sleep(Duration::from_millis(100))).await;
        drop(r);
        h2.drive(tokio::time::sleep(Duration::from_millis(250))).await;
        std::mem::forget((client, h2));
    };
    join(mock, cl).await;
}

#[tokio::test]
async fn t21_lost_wakeup_shared_send_task() {
    let (io, mut srv) = mock::new();
    let mock = async move {
        srv.assert_client_handshake_with_settings(frames::settings().max_concurrent_streams(1)).await;
        srv.recv_frame(frames::headers(1).request("POST", "https://a/").eos()).await;
        tokio::time::sleep(Duration::from_millis(200)).await;
        srv.send_frame(frames::headers(1).response(200).eos()).await;
        drain(&mut srv, 500, "srv").await;
    };
    let cl = async move {
        let (client, h2) = client::handshake(io).await.unwrap();
        let conn = tokio::spawn(async move { let r = h2.await; println!("conn result {:?}", r); });
        let mut c = client.ready().await.unwrap();
        let (_r1, _) = c.send_request(post(), true).unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        let mut c = c.ready().await.unwrap();
        let (_r3, mut s3) = c.send_request(post(), false).unwrap(); // pending open
        s3.reserve_capacity(10);
        // task B: wait for the handle to become ready again
        let b = tokio::spawn(async move {
            let t0 = std::time::Instant::now();
            let r = tokio::time::timeout(Duration::from_millis(800), c.ready()).await;
            println!("B ready() -> timed_out={} after {:?}", r.is_err(), t0.elapsed());
            if let Ok(Ok(x)) = r { std::mem::forget(x); }
        });
        tokio::time::sleep(Duration::from_millis(50)).await;
        // task A: poll capacity/reset on the same stream in a different task
        let a = tokio::spawn(async move {
            let r = tokio::time::timeout(Duration::from_millis(800), poll_fn(|cx| s3.poll_reset(cx))).await;
            println!("A poll_reset -> {:?}", r.is_err());
            std::mem::forget(s3);
        });
        let _ = join(a, b).await;
        std::mem::forget((_r1, _r3));
        conn.abort();
    };
    join(mock, cl).await;
}

#[tokio::test]
async fn t22_pp_unwrap_after_pushed_dropped() {
    let (io, mut c) = mock::new();
    let peer = async move {
        c.assert_server_handshake().await;
        c.send_bytes(&raw(1, END_HEADERS | END_STREAM, 1, &req_pseudo())).await;
        drain(&mut c, 300, "client").await;
    };
    let s = tokio::spawn(async move {
        let mut srv = server::Builder::new().max_concurrent_reset_streams(0).handshake::<_, Bytes>(io).await.unwrap();
        let (_req, mut resp) = srv.next().await.unwrap().unwrap();
        let pushed = resp.push_request(get()).unwrap();
        drop(pushed);
        tokio::select!{ _ = srv.next() => {}, _ = tokio::time::sleep(Duration::from_millis(200)) => {} };
        std::mem::forget((srv, resp));
    });
    peer.await;
    println!("server task {:?}", s.await.map_err(|e| e.is_panic()));
}

#[tokio::test]
async fn t23_parent_reset_before_pp_flush() {
    let (io, mut c) = mock::new();
    let peer = async move {
        c.assert_server_handshake().await;
        c.send_bytes(&raw(1, END_HEADERS | END_STREAM, 1, &req_pseudo())).await;
        drain(&mut c, 400, "client").await;
    };
    let s = async move {
        let mut srv = server::handshake(io).await.unwrap();
        let (_req, mut resp) = srv.next().await.unwrap().unwrap();
        let mut pushed = resp.push_request(get()).unwrap();
        resp.send_reset(Reason::CANCEL);
        let r = pushed.send_response(Response::builder().status(200).body(()).unwrap(), true);
        println!("pushed.send_response -> {:?}", r.is_ok());
        tokio::select!{ _ = srv.next() => {}, _ = tokio::time::sleep(Duration::from_millis(300)) => {} };
        println!("wired {} has_streams {}", srv.num_wired_streams(), srv.has_streams());
        std::mem::forget((srv, resp, pushed));
    };
    join(peer, s).await;
}

#[tokio::test]
async fn t24_pushed_head_content_length() {
    let (io, mut srv) = mock::new();
    let mock = async move {
        srv.assert_client_handshake().await;
        srv.recv_frame(frames::headers(1).request("GET", "https://a/").eos()).await;
        srv.send_frame(frames::headers(1).response(200)).await;
        srv.send_frame(frames::push_promise(1, 2).request("HEAD", "https://a/x")).await;
        srv.send_frame(frames::headers(2).response(200).field("content-length", "100").eos()).await;
        drain(&mut srv, 300, "srv").await;
    };
    let cl = async move {
        let (mut client, mut h2) = client::handshake(io).await.unwrap();
        let (mut resp, _) = client.send_request(get(), true).unwrap();
        let mut pushes = resp.push_promises();
        let p = h2.drive(pushes.next()).await.unwrap().unwrap();
        let (req, f) = p.into_parts();
        println!("pushed {:?}", req.method());
        let r = h2.drive(f).await;
        println!("pushed response -> {:?}", r.map(|x| x.status()));
        std::mem::forget((client, h2, resp, pushes));
    };
    join(mock, cl).await;
}

#[tokio::test]
async fn t24b_control_real_head() {
    let (io, mut srv) = mock::new();
    let mock = async move {
        srv.assert_client_handshake().await;
        srv.recv_frame(frames::headers(1).request("HEAD", "https://a/").eos()).await;
        srv.send_frame(frames::headers(1).response(200).field("content-length", "100").eos()).await;
        drain(&mut srv, 100, "srv").await;
    };
    let cl = async move {
        let (mut client, mut h2) = client::handshake(io).await.unwrap();
        let (resp, _) = client.send_request(Request::builder().method("HEAD").uri("https://a/").body(()).unwrap(), true).unwrap();
        println!("{:?}", h2.drive(resp).await.map(|x| x.status()));
        std::mem::forget((client, h2));
    };
    join(mock, cl).await;
}

#[tokio::test]
async fn t27_missing_status() {
    let (io, mut srv) = mock::new();
    let mock = async move {
        srv.assert_client_handshake().await;
        srv.recv_frame(frames::headers(1).request("GET", "https://a/").eos()).await;
        // HEADERS with only regular field + a request pseudo
        let mut b = lit(":path", "/x"); b.extend(lit("x-a", "1"));
        srv.send_bytes(&raw(1, END_HEADERS | END_STREAM, 1, &b)).await;
        drain(&mut srv, 100, "srv").await;
    };
    let cl = async move {
        let (mut client, mut h2) = client::handshake(io).await.unwrap();
        let (resp, _) = client.send_request(get(), true).unwrap();
        println!("response to status-less HEADERS: {:?}", h2.drive(resp).await.map(|x| (x.status(), x.headers().clone())));
        std::mem::forget((client, h2));
    };
    join(mock, cl).await;
}

#[tokio::test]
async fn t28_missing_required_pseudo() {
    let (io, mut c) = mock::new();
    let peer = async move {
        c.assert_server_handshake().await;
        // GET without :path, :authority
        c.send_bytes(&raw(1, END_HEADERS | END_STREAM, 1, &[0x82, 0x87])).await;
        // CONNECT without :authority
        let b = lit(":method", "CONNECT");
        c.send_bytes(&raw(1, END_HEADERS | END_STREAM, 3, &b)).await;
        // :path without leading slash
        let mut b = vec![0x82, 0x87]; b.extend(lit(":authority", "a")); b.extend(lit(":path", "foo"));
        c.send_bytes(&raw(1, END_HEADERS | END_STREAM, 5, &b)).await;
        // GET with :path * 
        let mut b = vec![0x82, 0x87]; b.extend(lit(":authority", "a")); b.extend(lit(":path", "*"));
        c.send_bytes(&raw(1, END_HEADERS | END_STREAM, 7, &b)).await;
        // value with leading space
        let mut b = req_pseudo(); b.extend(lit("x-a", " v "));
        c.send_bytes(&raw(1, END_HEADERS | END_STREAM, 9, &b)).await;
        drain(&mut c, 200, "client").await;
    };
    let s = async move {
        let mut srv = server::handshake(io).await.unwrap();
        while let Some(r) = srv.next().await {
            match r {
                Ok((req, mut resp)) => { println!("[srv] ACCEPT id={:?} {} uri={:?} {:?}", resp.stream_id(), req.method(), req.uri(), req.headers()); let _ = resp.send_response(Response::builder().status(200).body(()).unwrap(), true); }
                Err(e) => { println!("err {:?}", e); break; }
            }
        }
    };
    join(peer, s).await;
}

#[tokio::test]
async fn t29_push_invalid_leaks_child() {
    let (io, mut c) = mock::new();
    let peer = async move {
        c.assert_server_handshake().await;
        c.send_bytes(&raw(1, END_HEADERS | END_STREAM, 1, &req_pseudo())).await;
        drain(&mut c, 300, "client").await;
    };
    let s = async move {
        let mut srv = server::handshake(io).await.unwrap();
        let (_req, mut resp) = srv.next().await.unwrap().unwrap();
        println!("wired before {}", srv.num_wired_streams());
        for _ in 0..3 {
            let r = resp.push_request(post());
            println!("push POST -> err={:?}", r.as_ref().err().map(|e| e.to_string()));
        }
        tokio::select!{ _ = srv.next() => {}, _ = tokio::time::sleep(Duration::from_millis(100)) => {} };
        println!("wired after {}", srv.num_wired_streams());
        resp.send_response(Response::builder().status(200).body(()).unwrap(), true).unwrap();
        drop(resp); drop(_req);
        tokio::select!{ _ = srv.next() => {}, _ = tokio::time::sleep(Duration::from_millis(100)) => {} };
        println!("wired after stream finished {}", srv.num_wired_streams());
        std::mem::forget(srv);
    };
    join(peer, s).await;
}

#[tokio::test]
async fn t30_push_on_closed_parent() {
    let (io, mut c) = mock::new();
    let peer = async move {
        c.assert_server_handshake().await;
        c.send_bytes(&raw(1, END_HEADERS | END_STREAM, 1, &req_pseudo())).await;
        drain(&mut c, 300, "client").await;
    };
    let s = async move {
        let mut srv = server::handshake(io).await.unwrap();
        let (_req, mut resp) = srv.next().await.unwrap().unwrap();
        resp.send_response(Response::builder().status(200).body(()).unwrap(), true).unwrap();
        tokio::select!{ _ = srv.next() => {}, _ = tokio::time::sleep(Duration::from_millis(50)) => {} };
        let r = resp.push_request(get());
        println!("push after END_STREAM -> ok={:?}", r.is_ok());
        tokio::select!{ _ = srv.next() => {}, _ = tokio::time::sleep(Duration::from_millis(100)) => {} };
        std::mem::forget((srv, resp, r));
    };
    join(peer, s).await;
}

#[tokio::test]
async fn t31_reserve_truncates() {
    let (io, mut srv) = mock::new();
    let mock = async move { srv.assert_client_handshake().await; drain(&mut srv, 200, "srv").await; };
    let cl = async move {
        let (mut client, mut h2) = client::handshake(io).await.unwrap();
        let (_r, mut st) = client.send_request(post(), false).unwrap();
        h2.drive(tokio::time::sleep(Duration::from_millis(20))).await;
        st.reserve_capacity((1usize << 32) + 10);
        h2.drive(tokio::time::sleep(Duration::from_millis(20))).await;
        println!("capacity after reserve(2^32+10) = {}", st.capacity());
        std::mem::forget((client, h2, st, _r));
    };
    join(mock, cl).await;
}

#[tokio::test]
async fn t32_no_scheme_on_wire() {
    let (io, mut srv) = mock::new();
    let mock = async move {
        srv.assert_client_handshake().await;
        match srv.next().await { Some(Ok(h2::frame::Frame::Headers(h))) => println!("wire pseudo {:?}", h.into_parts().0), o => println!("{:?}", o) }
    };
    let cl = async move {
        let (mut client, mut h2) = client::handshake(io).await.unwrap();
        let req = Request::builder().method("GET").uri("example.com:8080").version(Version::HTTP_2).body(()).unwrap();
        let r = client.send_request(req, true);
        println!("send_request ok={}", r.is_ok());
        h2.drive(tokio::time::sleep(Duration::from_millis(50))).await;
        std::mem::forget((client, h2, r));
    };
    join(mock, cl).await;
}

#[tokio::test]
async fn t33_trailers_conn_header() {
    let (io, mut srv) = mock::new();
    let mock = async move { srv.assert_client_handshake().await; drain(&mut srv, 200, "srv").await; };
    let cl = async move {
        let (mut client, mut h2) = client::handshake(io).await.unwrap();
        let (_r, mut st) = client.send_request(post(), false).unwrap();
        let mut t = HeaderMap::new(); t.insert("connection", "close".parse().unwrap());
        println!("send_trailers with connection hdr -> {:?}", st.send_trailers(t).is_ok());
        h2.drive(tokio::time::sleep(Duration::from_millis(50))).await;
        std::mem::forget((client, h2, st, _r));
    };
    join(mock, cl).await;
}

#[tokio::test]
async fn t36_status_101_ignored() {
    let (io, mut srv) = mock::new();
    let mock = async move {
        srv.assert_client_handshake().await;
        srv.recv_frame(frames::headers(1).request("GET", "https://a/").eos()).await;
        srv.send_frame(frames::headers(1).response(101)).await;
        drain(&mut srv, 200, "srv").await;
    };
    let cl = async move {
        let (mut client, mut h2) = client::handshake(io).await.unwrap();
        let (resp, _) = client.send_request(get(), true).unwrap();
        let r = h2.drive(tokio::time::timeout(Duration::from_millis(150), resp)).await;
        println!("after 101 -> timed_out={}", r.is_err());
        std::mem::forget((client, h2, r));
    };
    join(mock, cl).await;
}

#[tokio::test]
async fn t21b_control_no_second_waiter() {
    let (io, mut srv) = mock::new();
    let mock = async move {
        srv.assert_client_handshake_with_settings(frames::settings().max_concurrent_streams(1)).await;
        srv.recv_frame(frames::headers(1).request("POST", "https://a/").eos()).await;
        tokio::time::sleep(Duration::from_millis(200)).await;
        srv.send_frame(frames::headers(1).response(200).eos()).await;
        drain(&mut srv, 500, "srv").await;
    };
    let cl = async move {
        let (client, h2) = client::handshake(io).await.unwrap();
        let conn = tokio::spawn(async move { let r = h2.await; println!("conn result {:?}", r); });
        let mut c = client.ready().await.unwrap();
        let (_r1, _) = c.send_request(post(), true).unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        let mut c = c.ready().await.unwrap();
        let (_r3, s3) = c.send_request(post(), false).unwrap();
        let b = tokio::spawn(async move {
            let t0 = std::time::Instant::now();
            let r = tokio::time::timeout(Duration::from_millis(800), c.ready()).await;
            println!("B ready() -> timed_out={} after {:?}", r.is_err(), t0.elapsed());
            if let Ok(Ok(x)) = r { std::mem::forget(x); }
        });
        let _ = b.await;
        std::mem::forget((_r1, _r3, s3));
        conn.abort();
    };
    join(mock, cl).await;
}

#[tokio::test]
async fn t37_unbounded_push_promises() {
    let (io, mut srv) = mock::new();
    let mock = async move {
        srv.assert_client_handshake().await;
        srv.recv_frame(frames::headers(1).request("GET", "https://a/").eos()).await;
        srv.send_frame(frames::headers(1).response(200)).await;
        for i in 0..5000u32 { srv.send_frame(frames::push_promise(1, 2 + 2 * i).request("GET", "https://a/x")).await; }
        drain(&mut srv, 300, "srv").await;
    };
    let cl = async move {
        let (mut client, mut h2) = client::Builder::new().max_concurrent_streams(10).handshake::<_, Bytes>(io).await.unwrap();
        let (resp, _) = client.send_request(get(), true).unwrap();
        h2.drive(tokio::time::sleep(Duration::from_millis(200))).await;
        println!("wired streams with max_concurrent_streams(10): {}", client.num_wired_streams());
        std::mem::forget((client, h2, resp));
    };
    join(mock, cl).await;
}

#[tokio::test]
async fn t38_rst_no_error_after_full_response() {
    let (io, mut srv) = mock::new();
    let mock = async move {
        srv.assert_client_handshake().await;
        srv.recv_frame(frames::headers(1).request("POST", "https://a/")).await;
        srv.send_frame(frames::headers(1).response(200)).await;
        srv.send_frame(frames::data(1, "complete").eos()).await;
        srv.send_frame(frames::reset(1).reason(Reason::NO_ERROR)).await;
        drain(&mut srv, 200, "srv").await;
    };
    let cl = async move {
        let (mut client, mut h2) = client::handshake(io).await.unwrap();
        let (resp, st) = client.send_request(post(), false).unwrap();
        let r = h2.drive(resp).await.unwrap();
        h2.drive(tokio::time::sleep(Duration::from_millis(50))).await;
        let mut b = r.into_body();
        println!("is_end_stream {}", b.is_end_stream());
        let d1 = b.data().await; println!("data1 {:?}", d1.map(|x| x.map(|y| y.len())));
        let d2 = b.data().await; println!("data2 {:?}", d2.map(|x| x.map(|y| y.len())));
        let t = b.trailers().await; println!("trailers {:?}", t);
        std::mem::forget((client, h2, st, b));
    };
    join(mock, cl).await;
}

#[tokio::test]
async fn t39_eof_after_full_response_bidi() {
    let (io, mut srv) = mock::new();
    let mock = async move {
        srv.assert_client_handshake().await;
        srv.recv_frame(frames::headers(1).request("POST", "https://a/")).await;
        srv.send_frame(frames::headers(1).response(200)).await;
        srv.send_frame(frames::data(1, "complete").eos()).await;
        srv.send_frame(frames::go_away(1)).await;
        // closes
    };
    let cl = async move {
        let (mut client, h2) = client::handshake(io).await.unwrap();
        let conn = tokio::spawn(async move { println!("conn {:?}", h2.await); });
        let (resp, st) = client.send_request(post(), false).unwrap();
        let r = resp.await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        let mut b = r.into_body();
        let d1 = b.data().await; println!("data1 {:?}", d1.map(|x| x.map(|y| y.len())));
        let d2 = b.data().await; println!("data2 {:?}", d2.map(|x| x.map(|y| y.len())));
        std::mem::forget((client, st, b));
    };
    join(mock, cl).await;
}

async fn reset_leak(block: bool) {
    let (io, mut c) = if block { mock::new_with_write_capacity(18) } else { mock::new() };
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<u8>();
    let (tx2, mut rx2) = tokio::sync::mpsc::unbounded_channel::<u8>();
    let peer = async move {
        c.assert_server_handshake().await;
        for i in 0..2u32 { c.send_bytes(&raw(1, END_HEADERS, 1 + 2*i, &req_pseudo())).await; }
        rx.recv().await; // resets issued
        tokio::time::sleep(Duration::from_millis(150)).await;
        // a frame to wake the server task past expiry while the socket stays blocked
        c.send_bytes(&raw(8, 0, 0, &[0,0,0,1])).await;
        tokio::time::sleep(Duration::from_millis(50)).await;
        c.unbounded_bytes().await;
        c.send_bytes(&raw(8, 0, 0, &[0,0,0,1])).await;
        tokio::time::sleep(Duration::from_millis(50)).await;
        drain(&mut c, 50, "client").await;
        tx2.send(1).unwrap();
        tokio::time::sleep(Duration::from_millis(5)).await;
        c.send_bytes(&raw(1, END_HEADERS, 5, &req_pseudo())).await;
        tokio::time::sleep(Duration::from_millis(15)).await;
        // a late DATA for the just reset stream 5: must be ignored for reset_stream_duration
        c.send_bytes(&raw(0, 0, 5, b"late")).await;
        tokio::time::sleep(Duration::from_millis(30)).await;
        drain(&mut c, 50, "client-late").await;
    };
    let s = async move {
        let mut srv = server::Builder::new().reset_stream_duration(Duration::from_millis(100)).max_concurrent_reset_streams(2).handshake::<_, Bytes>(io).await.unwrap();
        let mut v = vec![];
        for _ in 0..2 { v.push(srv.next().await.unwrap().unwrap()); }
        for (_r, s) in v.iter_mut() { s.send_reset(Reason::CANCEL); }
        tx.send(1).unwrap();
        // run the connection until told
        tokio::select!{ _ = srv.next() => {}, _ = rx2.recv() => {} };
        println!("wired before stream 5: {}", srv.num_wired_streams());
        let (_r, mut s5) = srv.next().await.unwrap().unwrap();
        s5.send_reset(Reason::CANCEL);
        drop(_r); drop(s5);
        drop(v.drain(..).collect::<Vec<_>>());
        tokio::select!{ _ = srv.next() => {}, _ = tokio::time::sleep(Duration::from_millis(5)) => {} };
        println!("wired after reset of 5: {}", srv.num_wired_streams());
        tokio::select!{ _ = srv.next() => {}, _ = tokio::time::sleep(Duration::from_millis(200)) => {} };
        std::mem::forget(srv);
        std::mem::forget(v);
    };
    join(peer, s).await;
}
#[tokio::test] async fn t34_blocked_writes() { reset_leak(true).await; }
#[tokio::test] async fn t34b_control() { reset_leak(false).await; }

async fn reset_leak2(big: usize) {
    let (io, mut c) = mock::new();
    let (tx2, mut rx2) = tokio::sync::mpsc::unbounded_channel::<u8>();
    let peer = async move {
        c.assert_server_handshake().await;
        c.send_bytes(&raw(1, END_HEADERS, 1, &req_pseudo())).await;
        tokio::time::sleep(Duration::from_millis(120)).await; // reset_stream_duration passes
        // wake the connection past expiry
        c.send_bytes(&raw(8, 0, 1, &[0,0,0,1])).await;
        tokio::time::sleep(Duration::from_millis(20)).await;
        c.send_bytes(&raw(8, 0, 1, &[0x7f,0,0,0])).await; // stream window
        c.send_bytes(&raw(8, 0, 0, &[0x7f,0,0,0])).await; // conn window
        tokio::time::sleep(Duration::from_millis(30)).await;
        drain(&mut c, 20, "client").await;
        c.send_bytes(&raw(1, END_HEADERS, 3, &req_pseudo())).await;
        tokio::time::sleep(Duration::from_millis(15)).await;
        c.send_bytes(&raw(0, 0, 3, b"late")).await;
        tokio::time::sleep(Duration::from_millis(30)).await;
        drain(&mut c, 30, "client-late").await;
    };
    let s = async move {
        let mut srv = server::Builder::new().reset_stream_duration(Duration::from_millis(100)).max_concurrent_reset_streams(1).handshake::<_, Bytes>(io).await.unwrap();
        let (r1, mut s1) = srv.next().await.unwrap().unwrap();
        let mut st = s1.send_response(Response::builder().status(200).body(()).unwrap(), false).unwrap();
        if big > 0 { st.send_data(Bytes::from(vec![0u8; big]), false).unwrap(); }
        tokio::select!{ _ = srv.next() => {}, _ = tokio::time::sleep(Duration::from_millis(30)) => {} };
        drop(r1); drop(s1); drop(st);
        let (r3, mut s3) = srv.next().await.unwrap().unwrap();
        s3.send_reset(Reason::CANCEL); drop(r3); drop(s3);
        tokio::select!{ _ = srv.next() => {}, _ = tokio::time::sleep(Duration::from_millis(100)) => {} };
        std::mem::forget(srv);
    };
    join(peer, s).await;
}
#[tokio::test] async fn t35_expiry_with_buffered_data() { reset_leak2(100_000).await; }
#[tokio::test] async fn t35b_control_no_buffered() { reset_leak2(0).await; }

#[tokio::test]
async fn t40_trailers_without_reading_data_hangs() {
    let (io, mut srv) = mock::new();
    let mock = async move {
        srv.assert_client_handshake().await;
        srv.recv_frame(frames::headers(1).request("GET", "https://a/").eos()).await;
        srv.send_frame(frames::headers(1).response(200)).await;
        srv.send_frame(frames::data(1, "x")).await;
        tokio::time::sleep(Duration::from_millis(50)).await;
        srv.send_frame(frames::headers(1).field("x-t", "1").eos()).await;
        drain(&mut srv, 800, "srv").await;
    };
    let cl = async move {
        let (mut client, h2) = client::handshake(io).await.unwrap();
        let conn = tokio::spawn(async move { let _ = h2.await; });
        let (resp, _) = client.send_request(get(), true).unwrap();
        let r = resp.await.unwrap();
        let mut b = r.into_body();
        tokio::time::sleep(Duration::from_millis(100)).await;
        // task only awaits trailers; the DATA event is not consumed
        let start = std::time::Instant::now();
        let mut fut = Box::pin(b.trailers());
        let woke = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let t = tokio::time::timeout(Duration::from_millis(400), fut.as_mut()).await;
        println!("trailers() with unread DATA -> timed_out={} after {:?}", t.is_err(), start.elapsed());
        drop(fut);
        std::mem::forget((client, b));
    };
    join(mock, cl).await;
}

#[tokio::test]
async fn t41_library_reset_does_not_wake_send_task() {
    let (io, mut c) = mock::new();
    let peer = async move {
        c.assert_server_handshake().await;
        let mut h = req_pseudo(); h.extend(lit("content-length", "1"));
        c.send_bytes(&raw(1, END_HEADERS, 1, &h)).await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        // violates content-length => h2 resets the stream itself
        c.send_bytes(&raw(0, 0, 1, b"hello")).await;
        drain(&mut c, 800, "client").await;
    };
    let s = async move {
        let mut srv = server::handshake(io).await.unwrap();
        let (_req, mut resp) = srv.next().await.unwrap().unwrap();
        let mut st = resp.send_response(Response::builder().status(200).body(()).unwrap(), false).unwrap();
        // handler parked on the stream from its own task, no timer of its own that would wake it
        let (txr, rxr) = tokio::sync::oneshot::channel::<String>();
        let (txc, rxc) = tokio::sync::oneshot::channel::<String>();
        tokio::spawn(async move {
            let r = poll_fn(|cx| st.poll_reset(cx)).await;
            let _ = txr.send(format!("{:?}", r));
            st.reserve_capacity(10);
        });
        tokio::spawn(async move { let (_a, _b) = (_req, resp); let _ = rxc; });
        let drive = async { loop { if srv.next().await.is_none() { break; } } };
        tokio::select!{ _ = drive => {}, r = tokio::time::timeout(Duration::from_millis(600), rxr) => println!("poll_reset after library reset -> {:?}", r) };
        std::mem::forget(srv);
    };
    join(peer, s).await;
}

#[tokio::test]
async fn t41b_control_remote_reset_wakes() {
    let (io, mut c) = mock::new();
    let peer = async move {
        c.assert_server_handshake().await;
        c.send_bytes(&raw(1, END_HEADERS, 1, &req_pseudo())).await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        c.send_bytes(&raw(3, 0, 1, &[0,0,0,8])).await;
        drain(&mut c, 800, "client").await;
    };
    let s = async move {
        let mut srv = server::handshake(io).await.unwrap();
        let (_req, mut resp) = srv.next().await.unwrap().unwrap();
        let mut st = resp.send_response(Response::builder().status(200).body(()).unwrap(), false).unwrap();
        let (txr, rxr) = tokio::sync::oneshot::channel::<String>();
        tokio::spawn(async move {
            let r = poll_fn(|cx| st.poll_reset(cx)).await;
            let _ = txr.send(format!("{:?}", r));
        });
        let drive = async { loop { if srv.next().await.is_none() { break; } } };
        tokio::select!{ _ = drive => {}, r = tokio::time::timeout(Duration::from_millis(600), rxr) => println!("poll_reset after remote reset -> {:?}", r) };
        std::mem::forget((srv, _req, resp));
    };
    join(peer, s).await;
}

#[tokio::test]
async fn t42_rst_on_idle_kills_conn_real_peers() {
    let (ca, sa) = tokio::io::duplex(1 << 20);
    let sv = tokio::spawn(async move {
        let mut srv = server::handshake(sa).await.unwrap();
        let mut held = vec![];
        loop {
            match srv.accept().await {
                Some(Ok((req, resp))) => { println!("[srv] got {:?}", req.uri()); held.push((req, resp)); }
                Some(Err(e)) => { println!("[srv] conn error {:?}", e); break; }
                None => { println!("[srv] closed"); break; }
            }
        }
    });
    let (client, h2) = client::handshake(ca).await.unwrap();
    let conn = tokio::spawn(async move { println!("[client conn] {:?}", h2.await); });
    let mut client = client.ready().await.unwrap();
    // an unrelated in-flight request
    let (r1, _s1) = client.send_request(post(), false).unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;
    let mut client = client.ready().await.unwrap();
    // cancel a second request right after issuing it, before the connection task ran
    let (_r3, mut s3) = client.send_request(post(), false).unwrap();
    s3.send_reset(Reason::CANCEL);
    let r = tokio::time::timeout(Duration::from_millis(300), r1).await;
    println!("unrelated request 1 -> {:?}", r.map(|x| x.map(|y| y.status())));
    let _ = sv.await;
    std::mem::forget((client, _s1, _r3, s3));
    conn.abort();
}

// pending-open variant
#[tokio::test]
async fn t42b_rst_on_pending_open() {
    let (io, mut srv) = mock::new();
    let mock = async move {
        srv.assert_client_handshake_with_settings(frames::settings().max_concurrent_streams(1)).await;
        srv.recv_frame(frames::headers(1).request("POST", "https://a/").eos()).await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        srv.send_frame(frames::headers(1).response(200).eos()).await;
        drain(&mut srv, 200, "srv").await;
    };
    let cl = async move {
        let (client, h2) = client::handshake(io).await.unwrap();
        let conn = tokio::spawn(async move { println!("conn {:?}", h2.await); });
        let mut c = client.ready().await.unwrap();
        let (_r1, _) = c.send_request(post(), true).unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        let (_r3, mut s3) = c.send_request(post(), false).unwrap();
        s3.send_reset(Reason::CANCEL);
        tokio::time::sleep(Duration::from_millis(300)).await;
        std::mem::forget((c, _r1, _r3, s3));
        conn.abort();
    };
    join(mock, cl).await;
}

#[tokio::test]
async fn t43_client_goaway_kills_clients_own_streams() {
    let (io, mut c) = mock::new();
    let peer = async move {
        c.assert_server_handshake().await;
        c.send_bytes(&raw(1, END_HEADERS | END_STREAM, 1, &req_pseudo())).await;
        tokio::time::sleep(Duration::from_millis(50)).await;
        // client: "no more pushes please": last_stream_id = 0 refers to server-initiated streams
        c.send_bytes(&raw(7, 0, 0, &[0,0,0,0, 0,0,0,0])).await;
        drain(&mut c, 300, "client").await;
    };
    let s = async move {
        let mut srv = server::handshake(io).await.unwrap();
        let (_req, mut resp) = srv.next().await.unwrap().unwrap();
        tokio::select!{ _ = srv.next() => {}, _ = tokio::time::sleep(Duration::from_millis(100)) => {} };
        let r = resp.send_response(Response::builder().status(200).body(()).unwrap(), true);
        println!("[srv] send_response after client GOAWAY(0): {:?}", r.map(|_| ()).map_err(|e| e.to_string()));
        tokio::select!{ _ = srv.next() => {}, _ = tokio::time::sleep(Duration::from_millis(100)) => {} };
        std::mem::forget((srv, resp, _req));
    };
    join(peer, s).await;
}

#[tokio::test]
async fn t43b_server_goaway_kills_pushed_stream() {
    let (io, mut srv) = mock::new();
    let mock = async move {
        srv.assert_client_handshake().await;
        srv.recv_frame(frames::headers(1).request("GET", "https://a/").eos()).await;
        srv.send_frame(frames::headers(1).response(200)).await;
        srv.send_frame(frames::push_promise(1, 2).request("GET", "https://a/x")).await;
        srv.send_frame(frames::headers(2).response(200)).await;
        // last_stream_id=1: all client initiated streams processed; says nothing about server's own stream 2
        srv.send_frame(frames::go_away(1)).await;
        tokio::time::sleep(Duration::from_millis(30)).await;
        srv.send_frame(frames::data(2, "push").eos()).await;
        srv.send_frame(frames::data(1, "").eos()).await;
        drain(&mut srv, 200, "srv").await;
    };
    let cl = async move {
        let (mut client, mut h2) = client::handshake(io).await.unwrap();
        let (mut resp, _) = client.send_request(get(), true).unwrap();
        let mut pushes = resp.push_promises();
        let p = h2.drive(pushes.next()).await.unwrap().unwrap();
        let (_, f) = p.into_parts();
        let r = h2.drive(f).await;
        println!("pushed response -> {:?}", r.as_ref().map(|x| x.status()));
        if let Ok(r) = r { println!("pushed body -> {:?}", h2.drive(util::concat(r.into_body())).await); }
        std::mem::forget((client, h2, resp, pushes));
    };
    join(mock, cl).await;
}

#[tokio::test]
async fn t44_error_codes() {
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("WINDOW_UPDATE stream=1 inc=0 (RFC: stream error)", raw(8, 0, 1, &[0,0,0,0])),
        ("PING len 7 (RFC: FRAME_SIZE_ERROR)", raw(6, 0, 0, &[0;7])),
        ("SETTINGS len 5 (RFC: FRAME_SIZE_ERROR)", raw(4, 0, 0, &[0;5])),
        ("SETTINGS ACK with payload (RFC: FRAME_SIZE_ERROR)", raw(4, 1, 0, &[0;6])),
        ("RST_STREAM len 3 (RFC: FRAME_SIZE_ERROR)", raw(3, 0, 1, &[0;3])),
        ("WINDOW_UPDATE len 3 (RFC: FRAME_SIZE_ERROR)", raw(8, 0, 0, &[0;3])),
        ("PRIORITY len 4 (RFC: stream FRAME_SIZE_ERROR)", raw(2, 0, 1, &[0;4])),
        ("SETTINGS INITIAL_WINDOW_SIZE 2^31 (RFC: FLOW_CONTROL_ERROR)", raw(4, 0, 0, &[0,4, 0x80,0,0,0])),
        ("HPACK index 0 (RFC: COMPRESSION_ERROR)", raw(1, END_HEADERS, 1, &[0x80])),
        ("HPACK truncated (RFC: COMPRESSION_ERROR)", raw(1, END_HEADERS, 1, &[0x82, 0x40])),
    ];
    for (name, bytes) in cases {
        let (io, mut c) = mock::new();
        let peer = async move {
            c.assert_server_handshake().await;
            c.send_bytes(&raw(1, END_HEADERS, 1, &req_pseudo())).await;
            c.send_bytes(&bytes).await;
            let mut r = vec![];
            while let Ok(Some(Ok(f))) = tokio::time::timeout(Duration::from_millis(50), c.next()).await { r.push(format!("{:?}", f)); }
            println!("{} => {:?}", name, r);
        };
        let s = async move {
            let mut srv = server::handshake(io).await.unwrap();
            let mut h = vec![];
            while let Ok(Some(Ok(x))) = tokio::time::timeout(Duration::from_millis(60), srv.next()).await { h.push(x); }
        };
        join(peer, s).await;
    }
}

#[tokio::test]
async fn t45_window_update_on_ignored_stream_after_goaway() {
    let (io, mut c) = mock::new();
    let peer = async move {
        c.assert_server_handshake().await;
        c.send_bytes(&raw(1, END_HEADERS, 1, &req_pseudo())).await;
        // GOAWAY(MAX) + PING
        let f = c.next().await; println!("[client] {:?}", f);
        let f = c.next().await; println!("[client] {:?}", f);
        c.send_bytes(&raw(6, 1, 0, &[0x0b, 0x7b, 0xa2, 0xf0, 0x8b, 0x9b, 0xfe, 0x54])).await; // ack
        let f = c.next().await; println!("[client] {:?}", f); // GOAWAY(1)
        // client raced a new request before seeing the GOAWAY
        c.send_bytes(&raw(1, END_HEADERS, 3, &req_pseudo())).await;
        c.send_bytes(&raw(8, 0, 3, &[0,1,0,0])).await;
        drain(&mut c, 300, "client").await;
    };
    let s = async move {
        let mut srv = server::handshake(io).await.unwrap();
        let (_req, mut resp) = srv.next().await.unwrap().unwrap();
        srv.graceful_shutdown();
        let drive = async { loop { match srv.next().await { None => { println!("[srv] closed"); break }, Some(Err(e)) => { println!("[srv] conn err {:?}", e); break }, Some(Ok(_)) => println!("[srv] accepted") } } };
        tokio::select!{ _ = drive => {}, _ = tokio::time::sleep(Duration::from_millis(250)) => {} };
        let r = resp.send_response(Response::builder().status(200).body(()).unwrap(), true);
        println!("[srv] response to in-flight stream 1 -> ok={}", r.is_ok());
        std::mem::forget((srv, resp, _req, r));
    };
    join(peer, s).await;
}
