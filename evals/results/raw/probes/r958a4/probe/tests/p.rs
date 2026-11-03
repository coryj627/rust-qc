#![allow(unused)]
use futures::StreamExt;
use h2_support::prelude::*;
use std::time::Duration;
use tokio::time::{sleep, timeout};

fn raw(kind: u8, flags: u8, id: u32, payload: &[u8]) -> Vec<u8> {
    let mut v = vec![];
    let l = payload.len();
    v.extend_from_slice(&[(l >> 16) as u8, (l >> 8) as u8, l as u8, kind, flags]);
    v.extend_from_slice(&id.to_be_bytes());
    v.extend_from_slice(payload);
    v
}
fn lit(n: &str, val: &str) -> Vec<u8> {
    let mut v = vec![0u8, n.len() as u8];
    v.extend_from_slice(n.as_bytes());
    v.push(val.len() as u8);
    v.extend_from_slice(val.as_bytes());
    v
}
// :method GET, :scheme https, :path /, :authority a.b
fn get_req() -> Vec<u8> {
    vec![0x82, 0x87, 0x84, 0x01, 3, b'a', b'.', b'b']
}
fn resp200() -> Vec<u8> { vec![0x88] }

async fn drain(name: &str, srv: &mut mock::Handle, ms: u64) {
    loop {
        match timeout(Duration::from_millis(ms), srv.next()).await {
            Ok(Some(Ok(f))) => println!("  [{name}] peer saw: {:?}", f),
            Ok(Some(Err(e))) => { println!("  [{name}] peer codec err: {:?}", e); break }
            Ok(None) => { println!("  [{name}] peer saw EOF"); break }
            Err(_) => { println!("  [{name}] (quiet)"); break }
        }
    }
}

/// h2 server under test; `chunks` raw bytes are written by the mock client.
async fn srv_case(name: &'static str, chunks: Vec<Vec<u8>>) {
    println!("--- {name}");
    let (io, mut cli) = mock::new();
    let task = tokio::spawn(async move {
        let mut conn = server::handshake(io).await.unwrap();
        let mut keep = vec![];
        while let Some(r) = conn.accept().await {
            match r {
                Ok((req, resp)) => {
                    println!("  [{name}] server ACCEPTED {:?} {:?} hdrs={:?}", req.method(), req.uri(), req.headers());
                    keep.push((req, resp));
                }
                Err(e) => { println!("  [{name}] accept err {:?}", e); break }
            }
        }
        println!("  [{name}] server loop ended");
    });
    cli.assert_server_handshake().await;
    for c in chunks { cli.send_bytes(&c).await; }
    drain(name, &mut cli, 200).await;
    task.abort();
    let _ = task.await;
}

#[tokio::test]
async fn p7_malformed_lost_across_needmore() {
    let mut a = get_req();
    a.extend(lit("connection", "close"));
    let mut whole = a.clone();
    whole.extend(lit("foo", "bar"));
    srv_case("p7-control-single-frame", vec![raw(1, 5, 1, &whole)]).await;
    // split such that first fragment ends mid-header
    let mut first = a.clone();
    first.extend_from_slice(&[0u8, 3, b'f', b'o']);
    let second = [b'o', 3, b'b', b'a', b'r'];
    srv_case("p7-split", vec![raw(1, 1, 1, &first), raw(9, 4, 1, &second)]).await;
}

#[tokio::test]
async fn p8_malformed_with_continuation() {
    let mut a = get_req();
    a.extend(lit("connection", "close"));
    let c = lit("foo", "bar");
    srv_case("p8", vec![raw(1, 1, 1, &a), raw(9, 4, 1, &c)]).await;
}

#[tokio::test]
async fn p10_missing_pseudo() {
    srv_case("p10-no-path-no-auth", vec![raw(1, 5, 1, &[0x82, 0x87])]).await;
    let mut no_path = vec![0x82, 0x87, 0x01, 3, b'a', b'.', b'b'];
    srv_case("p10-no-path-with-auth", vec![raw(1, 5, 1, &no_path)]).await;
    // CONNECT no authority: literal :method (idx 2) value CONNECT
    let mut c = vec![0x02, 7];
    c.extend_from_slice(b"CONNECT");
    srv_case("p10-connect-no-authority", vec![raw(1, 5, 1, &c)]).await;
    // :path without leading slash: idx 4 literal
    let mut p = vec![0x82, 0x87, 0x04, 3];
    p.extend_from_slice(b"foo");
    srv_case("p10-path-no-slash", vec![raw(1, 5, 1, &p)]).await;
    // :path * on GET
    let p = vec![0x82, 0x87, 0x04, 1, b'*'];
    srv_case("p10-star-on-get", vec![raw(1, 5, 1, &p)]).await;
}

#[tokio::test]
async fn p11_trailers_pseudo() {
    let mut t = vec![0x84];
    t.extend(lit("x", "y"));
    srv_case("p11", vec![raw(1, 4, 1, &get_req()), raw(1, 5, 1, &t)]).await;
}

#[tokio::test]
async fn p22_dup_content_length() {
    let mut h = get_req();
    h.extend(lit("content-length", "5"));
    h.extend(lit("content-length", "100"));
    srv_case("p22", vec![raw(1, 4, 1, &h), raw(0, 1, 1, b"hello")]).await;
    let mut h = get_req();
    h.extend(lit("content-length", ""));
    srv_case("p22-empty-cl", vec![raw(1, 5, 1, &h)]).await;
}

#[tokio::test]
async fn p24_dup_rst() {
    let mut chunks = vec![raw(1, 4, 1, &get_req())];
    let mut big = vec![];
    for _ in 0..25 { big.extend(raw(3, 0, 1, &[0, 0, 0, 8])); }
    let mut all = chunks.pop().unwrap();
    all.extend(big);
    srv_case("p24", vec![all]).await;
}

#[tokio::test]
async fn p29_p30_hpack() {
    let mut a = get_req();
    a.extend_from_slice(&[0x40, 0x00, 0x01, b'x']);
    srv_case("p29-empty-name-split", vec![raw(1, 1, 1, &a), raw(9, 4, 1, &[])]).await;
    srv_case("p29-empty-name-whole", vec![raw(1, 5, 1, &a)]).await;
    srv_case("p30-size-update-mid-block-split", vec![raw(1, 1, 1, &get_req()), raw(9, 4, 1, &[0x20])]).await;
    let mut w = get_req(); w.push(0x20);
    srv_case("p30-size-update-mid-block-whole", vec![raw(1, 5, 1, &w)]).await;
    let mut bad = get_req(); bad.push(0x80);
    srv_case("p31-bad-index-goaway-code", vec![raw(1, 5, 1, &bad)]).await;
    let mut up = vec![0x82,0x87,0x84,0x01,3,b'a',b'.',b'b'];
    up.extend(vec![0, 3, b'F', b'O', b'O', 1, b'x']);
    srv_case("p31-uppercase-name", vec![raw(1, 5, 1, &up)]).await;
}

#[tokio::test]
async fn p32_error_codes() {
    srv_case("ping-short", vec![raw(6, 0, 0, &[0; 7])]).await;
    srv_case("settings-len7", vec![raw(4, 0, 0, &[0; 7])]).await;
    srv_case("settings-ack-with-payload", vec![raw(4, 1, 0, &[0; 6])]).await;
    srv_case("wu-short", vec![raw(8, 0, 0, &[0; 3])]).await;
    srv_case("rst-short", vec![raw(1,5,1,&get_req()), raw(3, 0, 1, &[0; 3])]).await;
    srv_case("priority-short", vec![raw(2, 0, 1, &[0; 4])]).await;
    srv_case("settings-iws-too-big", vec![raw(4, 0, 0, &[0, 4, 0xff, 0xff, 0xff, 0xff])]).await;
    srv_case("wu-zero-on-stream", vec![raw(1,4,1,&get_req()), raw(8, 0, 1, &[0; 4])]).await;
    srv_case("goaway-on-stream-1", vec![raw(1,4,1,&get_req()), raw(7, 0, 1, &[0; 8])]).await;
    srv_case("data-after-es", vec![raw(1,5,1,&get_req()), raw(0, 0, 1, b"x")]).await;
    srv_case("headers-after-es", vec![raw(1,5,1,&get_req()), raw(1, 5, 1, &[0x40,1,b'x',1,b'y'])]).await;
    srv_case("settings-frame-max-too-small", vec![raw(4, 0, 0, &[0, 5, 0, 0, 0, 1])]).await;
    srv_case("padded-data-all-padding-oversize", vec![raw(1,4,1,&get_req()), raw(0, 8, 1, &[5, 0])]).await;
}

/// h2 client under test.
async fn client_setup() -> (client::SendRequest<Bytes>, client::Connection<mock::Mock, Bytes>, mock::Handle) {
    let (io, mut srv) = mock::new();
    let hs = tokio::spawn(async move { srv.assert_client_handshake().await; srv });
    let (c, h2) = client::handshake(io).await.unwrap();
    let srv = hs.await.unwrap();
    (c, h2, srv)
}

fn get() -> Request<()> {
    Request::builder().method("GET").uri("https://a.b/").body(()).unwrap()
}

#[tokio::test]
async fn p1_pushed_1xx_double_count() {
    let (io, mut srv) = mock::new();
    let task = tokio::spawn(async move {
        let (mut c, h2) = client::handshake(io).await.unwrap();
        let (mut resp, _s) = c.send_request(get(), true).unwrap();
        let conn = tokio::spawn(async move { println!("  conn result {:?}", h2.await); });
        let mut pp = resp.push_promises();
        let r = pp.push_promise().await;
        println!("  push {:?}", r.as_ref().map(|x| x.as_ref().map(|y| y.request().uri().clone()).map_err(|e| e.to_string())));
        let (rq, pr) = r.unwrap().unwrap().into_parts();
        let rr = pr.await;
        println!("  pushed resp {:?}", rr.map(|r| r.status()));
        println!("  join conn {:?}", conn.await.map_err(|e| e.to_string()));
    });
    srv.assert_client_handshake().await;
    let _ = srv.next().await; // request
    srv.send_frame(frames::push_promise(1, 2).request("GET", "https://a.b/x")).await;
    srv.send_frame(frames::headers(2).response(103)).await;
    srv.send_frame(frames::headers(2).response(200).eos()).await;
    srv.send_frame(frames::headers(1).response(200).eos()).await;
    println!("--- p1"); 
    let r = timeout(Duration::from_secs(1), task).await;
    println!("  p1 task {:?}", r.map(|x| x.map_err(|e| e.to_string())));
}

#[tokio::test]
async fn p2_pushed_over_concurrency() {
    let (io, mut srv) = mock::new();
    let task = tokio::spawn(async move {
        let (mut c, h2) = client::Builder::new().max_concurrent_streams(1).handshake::<_, Bytes>(io).await.unwrap();
        let (mut resp, _s) = c.send_request(get(), true).unwrap();
        println!("  conn result {:?}", h2.await);
        drop(resp);
    });
    srv.assert_client_handshake_with_settings(frames::settings()).await;
    let _ = srv.next().await; // settings ack / request
    srv.send_frame(frames::push_promise(1, 2).request("GET", "https://a.b/x")).await;
    srv.send_frame(frames::push_promise(1, 4).request("GET", "https://a.b/y")).await;
    srv.send_frame(frames::headers(2).response(200)).await;
    srv.send_frame(frames::headers(4).response(200)).await;
    println!("--- p2");
    let r = timeout(Duration::from_millis(500), task).await;
    println!("  p2 task {:?}", r.map(|x| x.map_err(|e| e.to_string())));
    drain("p2", &mut srv, 100).await;
}

#[tokio::test]
async fn p3a_drop_push_handle_no_reset_slots() {
    println!("--- p3a");
    let (io, mut cli) = mock::new();
    let task = tokio::spawn(async move {
        let mut conn = server::Builder::new().max_concurrent_reset_streams(0).handshake::<_, Bytes>(io).await.unwrap();
        let (_req, mut respond) = conn.accept().await.unwrap().unwrap();
        let pushed = respond.push_request(Request::builder().method("GET").uri("https://a.b/x").body(()).unwrap()).unwrap();
        drop(pushed);
        let r = conn.accept().await;
        println!("  accept {:?}", r.map(|x| x.map(|_| ()).map_err(|e| e.to_string())));
    });
    cli.assert_server_handshake().await;
    cli.send_bytes(&raw(1, 5, 1, &get_req())).await;
    let r = timeout(Duration::from_millis(500), task).await;
    println!("  p3a task {:?}", r.map(|x| x.map_err(|e| e.to_string())));
}

#[tokio::test]
async fn p3b_drop_push_handle_default_config_flow_blocked() {
    println!("--- p3b");
    let (io, mut cli) = mock::new();
    let task = tokio::spawn(async move {
        let mut conn = server::handshake(io).await.unwrap();
        let (_req, mut respond) = conn.accept().await.unwrap().unwrap();
        let c = tokio::spawn(async move {
            let r = futures::future::poll_fn(|cx| conn.poll_closed(cx)).await;
            println!("  conn closed {:?}", r);
        });
        let mut s = respond.send_response(Response::new(()), false).unwrap();
        s.send_data(Bytes::from(vec![0u8; 100_000]), false).unwrap();
        let pushed = respond.push_request(Request::builder().method("GET").uri("https://a.b/x").body(()).unwrap()).unwrap();
        drop(pushed);
        println!("  conn join: {:?}", c.await.map_err(|e| e.to_string()));
        drop(s);
    });
    cli.assert_server_handshake().await;
    cli.send_bytes(&raw(1, 5, 1, &get_req())).await;
    drain("p3b", &mut cli, 300).await;
    sleep(Duration::from_millis(1200)).await;
    // keep-alive frame so the connection task polls and expires the reset stream
    cli.send_bytes(&raw(6, 0, 0, &[1; 8])).await;
    sleep(Duration::from_millis(50)).await;
    cli.send_bytes(&raw(8, 0, 0, &[0, 1, 0, 0])).await;
    cli.send_bytes(&raw(8, 0, 1, &[0, 1, 0, 0])).await;
    let r = timeout(Duration::from_millis(500), task).await;
    println!("  p3b task {:?}", r.map(|x| x.map_err(|e| e.to_string())));
}

#[tokio::test]
async fn p4_reset_before_open() {
    println!("--- p4");
    let (io, mut srv) = mock::new();
    let task = tokio::spawn(async move {
        let (mut c, h2) = client::handshake(io).await.unwrap();
        let (resp, mut s) = c.send_request(get(), false).unwrap();
        s.send_reset(Reason::CANCEL);
        println!("  conn {:?}", h2.await);
        drop((resp, s, c));
    });
    srv.assert_client_handshake().await;
    drain("p4", &mut srv, 200).await;
    task.abort();
}

#[tokio::test]
async fn p5_padding_flow_control() {
    println!("--- p5");
    let (io, mut srv) = mock::new();
    let task = tokio::spawn(async move {
        let (mut c, h2) = client::handshake(io).await.unwrap();
        let (resp, _s) = c.send_request(get(), true).unwrap();
        let conn = tokio::spawn(async move { let _ = h2.await; });
        let mut r = resp.await.unwrap().into_body();
        let mut tot = 0;
        while let Some(d) = r.data().await {
            let d = d.unwrap();
            tot += d.len();
            r.flow_control().release_capacity(d.len()).unwrap();
        }
        println!("  client read {tot} bytes");
        sleep(Duration::from_secs(5)).await;
        drop(conn); drop(c);
    });
    srv.assert_client_handshake().await;
    let _ = srv.next().await;
    srv.send_bytes(&raw(1, 4, 1, &resp200())).await;
    // 52 frames: 1000 payload + 255 pad + 1 pad-len = 1256 wire flow controlled octets each
    let mut sent_fc = 0usize;
    for _ in 0..52 {
        let mut p = vec![255u8];
        p.extend(vec![b'x'; 1000]);
        p.extend(vec![0u8; 255]);
        sent_fc += p.len();
        srv.send_bytes(&raw(0, 8, 1, &p)).await;
    }
    println!("  flow-controlled octets sent per RFC: {sent_fc}");
    drain("p5", &mut srv, 200).await;
    task.abort();

    println!("--- p5b all-padding exhaustion");
    let (io, mut srv) = mock::new();
    let task = tokio::spawn(async move {
        let (mut c, h2) = client::handshake(io).await.unwrap();
        let (resp, _s) = c.send_request(get(), true).unwrap();
        let conn = tokio::spawn(async move { println!(" conn {:?}", h2.await); });
        let mut r = resp.await.unwrap().into_body();
        while let Some(d) = r.data().await { let d = d.unwrap(); }
        sleep(Duration::from_secs(5)).await;
        drop(conn); drop(c);
    });
    srv.assert_client_handshake().await;
    let _ = srv.next().await;
    srv.send_bytes(&raw(1, 4, 1, &resp200())).await;
    // send well over the 65535 window: 2000 frames of 256 flow-controlled octets
    for _ in 0..2000 {
        let mut p = vec![255u8]; p.extend(vec![0u8; 255]);
        srv.send_bytes(&raw(0, 8, 1, &p)).await;
    }
    println!("  sent 512000 flow-controlled octets vs window 65535");
    drain("p5b", &mut srv, 200).await;
    task.abort();
}
