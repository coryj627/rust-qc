#![allow(unused)]
use futures::StreamExt;
use h2_support::prelude::*;
use h2::frame;
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
fn get_req() -> Vec<u8> { vec![0x82, 0x87, 0x84, 0x01, 3, b'a', b'.', b'b'] }
fn get() -> Request<()> { Request::builder().method("GET").uri("https://a.b/").body(()).unwrap() }
fn push_req() -> Request<()> { Request::builder().method("GET").uri("https://a.b/x").body(()).unwrap() }

async fn drain(name: &str, srv: &mut mock::Handle, ms: u64) {
    loop {
        match timeout(Duration::from_millis(ms), srv.next()).await {
            Ok(Some(Ok(f))) => println!("  [{name}] peer saw: {:?}", f),
            Ok(Some(Err(e))) => { println!("  [{name}] peer codec err: {:?}", e); break }
            Ok(None) => { println!("  [{name}] peer saw EOF"); break }
            Err(_) => { break }
        }
    }
}

#[tokio::test]
async fn q9_response_without_status() {
    let (io, mut srv) = mock::new();
    let task = tokio::spawn(async move {
        let (mut c, h2) = client::handshake(io).await.unwrap();
        let (resp, _s) = c.send_request(get(), true).unwrap();
        let (resp3, _s3) = c.send_request(get(), true).unwrap();
        let conn = tokio::spawn(async move { println!("  conn {:?}", h2.await) });
        match resp.await { Ok(r) => println!("  no :status -> client saw {:?} hdrs={:?}", r.status(), r.headers()), Err(e) => println!("  err {e}") }
        match resp3.await { Ok(r) => println!("  :status + :path + :method -> client saw {:?}", r.status()), Err(e) => println!("  err3 {e}") }
        drop(conn);
    });
    srv.assert_client_handshake().await;
    let _ = srv.next().await; let _ = srv.next().await;
    let mut p = lit("x", "y");
    srv.send_bytes(&raw(1, 5, 1, &p)).await;
    let mut p = vec![0x88, 0x84, 0x82];
    srv.send_bytes(&raw(1, 5, 3, &p)).await;
    let _ = timeout(Duration::from_secs(1), task).await;
}

#[tokio::test]
async fn q12_pushed_head() {
    let (io, mut srv) = mock::new();
    let task = tokio::spawn(async move {
        let (mut c, h2) = client::handshake(io).await.unwrap();
        let (mut resp, _s) = c.send_request(get(), false).unwrap();
        let mut pp = resp.push_promises();
        let conn = tokio::spawn(async move { println!("  conn {:?}", h2.await) });
        let x = pp.push_promise().await.unwrap().unwrap();
        let (rq, pr) = x.into_parts();
        println!("  promised {:?}", rq.method());
        match pr.await { Ok(r) => println!("  pushed resp {:?}", r.status()), Err(e) => println!("  pushed resp ERR {e}") }
        drop(conn);
    });
    srv.assert_client_handshake().await;
    let _ = srv.next().await;
    srv.send_frame(frames::push_promise(1, 2).request("HEAD", "https://a.b/x")).await;
    srv.send_frame(frames::headers(2).response(200).field("content-length", "10").eos()).await;
    let _ = timeout(Duration::from_secs(1), task).await;
    drain("q12", &mut srv, 100).await;
}

#[tokio::test]
async fn q13_trailers_for_forgotten_stream() {
    let (io, mut cli) = mock::new();
    let task = tokio::spawn(async move {
        let mut conn = server::Builder::new().max_concurrent_reset_streams(0).handshake::<_, Bytes>(io).await.unwrap();
        let (a, mut respond) = conn.accept().await.unwrap().unwrap();
        let (b, _r2) = conn.accept().await.unwrap().unwrap();
        respond.send_reset(Reason::CANCEL);
        drop(a); drop(respond);
        let r = conn.accept().await;
        println!("  accept {:?}", r.map(|x| x.map(|_| ()).map_err(|e| e.to_string())));
    });
    cli.assert_server_handshake().await;
    cli.send_bytes(&raw(1, 4, 1, &get_req())).await;
    cli.send_bytes(&raw(1, 4, 3, &[0xbe, 0xbe, 0xbe, 0xbe][..0].iter().copied().chain(get_req()).collect::<Vec<u8>>())).await;
    drain("q13", &mut cli, 100).await;
    println!("  client sends in-flight DATA, then trailers, for reset stream 1");
    cli.send_bytes(&raw(0, 0, 1, b"x")).await;
    drain("q13", &mut cli, 100).await;
    cli.send_bytes(&raw(1, 5, 1, &lit("t", "v"))).await;
    drain("q13", &mut cli, 100).await;
    let _ = timeout(Duration::from_millis(200), task).await;
}

#[tokio::test]
async fn q14_late_frames_kill_connection() {
    let (io, mut srv) = mock::new();
    let task = tokio::spawn(async move {
        let (mut c, h2) = client::Builder::new().max_concurrent_reset_streams(0).handshake::<_, Bytes>(io).await.unwrap();
        let conn = tokio::spawn(async move { println!("  conn {:?}", h2.await) });
        let (resp, s) = c.send_request(get(), false).unwrap();
        tokio::task::yield_now().await;
        drop(resp); drop(s);
        // a second, healthy request
        let (resp2, _s2) = c.send_request(get(), true).unwrap();
        let r = resp2.await;
        println!("  healthy request result {:?}", r.map(|r| r.status()).map_err(|e| e.to_string()));
        drop(conn);
    });
    srv.assert_client_handshake().await;
    let mut n = 0;
    drain("q14", &mut srv, 100).await;
    for i in 0..1100 {
        srv.send_bytes(&raw(0, 0, 1, b"x")).await;
        match timeout(Duration::from_millis(100), srv.next()).await {
            Ok(Some(Ok(frame::Frame::Reset(r)))) => { n += 1; }
            Ok(Some(Ok(f))) => { println!("  after {i} late DATA frames ({n} STREAM_CLOSED) peer saw: {:?}", f); break }
            other => { println!("  other {:?}", other.map(|x| x.map(|y| y.map(|_| ()).map_err(|_| ())))); break }
        }
    }
    let _ = timeout(Duration::from_millis(300), task).await;
}


#[tokio::test]
async fn q15_body_drop_keeps_window() {
    let (io, mut cli) = mock::new();
    let task = tokio::spawn(async move {
        let mut conn = server::handshake(io).await.unwrap();
        let (req, respond) = conn.accept().await.unwrap().unwrap();
        // let the 65535 bytes arrive, then drop the body while keeping SendResponse.
        let body = req.into_body();
        let a = async { sleep(Duration::from_millis(100)).await; drop(body); println!("  server dropped RecvStream, keeps SendResponse"); sleep(Duration::from_secs(5)).await; drop(respond); };
        let b = async { while let Some(r) = conn.accept().await {
            match r { Ok((q, _)) => println!("  accepted {:?}", q.uri()), Err(e) => { println!("  accept err {e}"); break } } } };
        tokio::join!(a, b);
    });
    cli.assert_server_handshake().await;
    cli.send_bytes(&raw(1, 4, 1, &get_req())).await;
    let chunk = vec![b'x'; 16383];
    for _ in 0..4 { cli.send_bytes(&raw(0, 0, 1, &chunk)).await; }
    cli.send_bytes(&raw(0, 0, 1, &vec![b'x'; 3])).await;
    println!("  sent 65535 octets of body: connection window exhausted from client view");
    drain("q15", &mut cli, 500).await;
    println!("  (window updates, if any, are printed above)");
    task.abort();
}

#[tokio::test]
async fn q16_trailers_hang() {
    let (io, mut srv) = mock::new();
    let task = tokio::spawn(async move {
        let (mut c, h2) = client::handshake(io).await.unwrap();
        let (resp, _s) = c.send_request(get(), true).unwrap();
        let conn = tokio::spawn(async move { let _ = h2.await; });
        let mut b = resp.await.unwrap().into_body();
        sleep(Duration::from_millis(50)).await;
        let r = timeout(Duration::from_millis(500), b.trailers()).await;
        println!("  trailers() without draining data: {:?}", r.map(|x| x.map_err(|e| e.to_string())).map_err(|_| "TIMED OUT"));
        drop(conn); drop(c);
    });
    srv.assert_client_handshake().await;
    let _ = srv.next().await;
    srv.send_frame(frames::headers(1).response(200)).await;
    srv.send_frame(frames::data(1, "hi")).await;
    srv.send_frame(frames::headers(1).field("t", "v").eos()).await;
    let _ = timeout(Duration::from_secs(2), task).await;
}

#[tokio::test]
async fn q17a_pp_after_user_reset() {
    let (io, mut srv) = mock::new();
    let task = tokio::spawn(async move {
        let (mut c, h2) = client::handshake(io).await.unwrap();
        let (resp, mut s) = c.send_request(get(), false).unwrap();
        let (resp2, _s2) = c.send_request(get(), true).unwrap();
        let conn = tokio::spawn(async move { println!("  conn {:?}", h2.await) });
        sleep(Duration::from_millis(50)).await;
        s.send_reset(Reason::CANCEL);
        println!("  resp2 {:?}", resp2.await.map(|r| r.status()).map_err(|e| e.to_string()));
        println!("  join {:?}", conn.await.map_err(|e| e.to_string()));
        drop((resp, s, c));
    });
    srv.assert_client_handshake().await;
    let _ = srv.next().await; let _ = srv.next().await;
    sleep(Duration::from_millis(100)).await; // peer's PUSH_PROMISE crosses our RST_STREAM on the wire
    srv.send_frame(frames::push_promise(1, 2).request("GET", "https://a.b/x")).await;
    srv.send_frame(frames::headers(2).response(200).eos()).await;
    srv.send_frame(frames::headers(3).response(200).eos()).await;
    let _ = timeout(Duration::from_millis(500), task).await;
    drain("q17a", &mut srv, 100).await;
}

#[tokio::test]
async fn q17b_pp_after_handle_drop_scheduled() {
    let (io, mut srv) = mock::new();
    let task = tokio::spawn(async move {
        let (mut c, mut h2) = client::handshake(io).await.unwrap();
        let (resp, s) = c.send_request(get(), false).unwrap();
        let (resp2, _s2) = c.send_request(get(), true).unwrap();
        // flush headers
        let _ = timeout(Duration::from_millis(100), &mut h2).await;
        // wait for peer frame to be in the pipe, then drop without polling.
        sleep(Duration::from_millis(150)).await;
        drop(resp); drop(s);
        let conn = tokio::spawn(async move { println!("  conn {:?}", h2.await) });
        println!("  resp2 {:?}", resp2.await.map(|r| r.status()).map_err(|e| e.to_string()));
        drop(conn); drop(c);
    });
    srv.assert_client_handshake().await;
    let _ = srv.next().await; let _ = srv.next().await;
    sleep(Duration::from_millis(130)).await; // connection no longer being polled
    srv.send_frame(frames::push_promise(1, 2).request("GET", "https://a.b/x")).await;
    let _ = timeout(Duration::from_millis(600), task).await;
    drain("q17b", &mut srv, 100).await;
}

#[tokio::test]
async fn q17c_pp_after_forgotten() {
    let (io, mut srv) = mock::new();
    let task = tokio::spawn(async move {
        let (mut c, h2) = client::Builder::new().max_concurrent_reset_streams(0).handshake::<_, Bytes>(io).await.unwrap();
        let (resp, s) = c.send_request(get(), false).unwrap();
        let (resp2, _s2) = c.send_request(get(), true).unwrap();
        let conn = tokio::spawn(async move { println!("  conn {:?}", h2.await) });
        sleep(Duration::from_millis(50)).await;
        drop(resp); drop(s);
        println!("  resp2 {:?}", resp2.await.map(|r| r.status()).map_err(|e| e.to_string()));
        drop(conn); drop(c);
    });
    srv.assert_client_handshake().await;
    let _ = srv.next().await; let _ = srv.next().await;
    sleep(Duration::from_millis(100)).await;
    drain("q17c", &mut srv, 50).await;
    srv.send_frame(frames::push_promise(1, 2).request("GET", "https://a.b/x")).await;
    let _ = timeout(Duration::from_millis(500), task).await;
    drain("q17c", &mut srv, 100).await;
}

#[tokio::test]
async fn q19_failed_push_leaks_stream() {
    let (io, mut cli) = mock::new();
    let task = tokio::spawn(async move {
        let mut conn = server::handshake(io).await.unwrap();
        let (_q, mut respond) = conn.accept().await.unwrap().unwrap();
        println!("  wired before: {}", conn.num_wired_streams());
        for _ in 0..100 {
            let bad = Request::builder().method("POST").uri("https://a.b/x").body(()).unwrap();
            assert!(respond.push_request(bad).is_err());
        }
        println!("  wired after 100 rejected push_request: {}", conn.num_wired_streams());
    });
    cli.assert_server_handshake().await;
    cli.send_bytes(&raw(1, 5, 1, &get_req())).await;
    let _ = timeout(Duration::from_millis(300), task).await;
}

#[tokio::test]
async fn q21_reserve_capacity_truncates() {
    let (io, mut srv) = mock::new();
    let task = tokio::spawn(async move {
        let (mut c, h2) = client::handshake(io).await.unwrap();
        let (_resp, mut s) = c.send_request(get(), false).unwrap();
        let conn = tokio::spawn(async move { let _ = h2.await; });
        sleep(Duration::from_millis(30)).await;
        s.reserve_capacity((1usize << 32) + 5);
        println!("  reserve_capacity(2^32+5) -> capacity() = {}", s.capacity());
        drop(conn); drop(s); drop(c);
    });
    srv.assert_client_handshake().await;
    let _ = srv.next().await;
    let _ = timeout(Duration::from_millis(300), task).await;
}

#[tokio::test]
async fn q23_304_cl_empty_data() {
    let (io, mut srv) = mock::new();
    let task = tokio::spawn(async move {
        let (mut c, h2) = client::handshake(io).await.unwrap();
        let (resp, _s) = c.send_request(get(), true).unwrap();
        let conn = tokio::spawn(async move { println!("  conn {:?}", h2.await) });
        let r = resp.await.unwrap();
        println!("  status {:?}", r.status());
        let mut b = r.into_body();
        println!("  body {:?}", b.data().await.map(|x| x.map(|y| y.len()).map_err(|e| e.to_string())));
        drop(conn); drop(c);
    });
    srv.assert_client_handshake().await;
    let _ = srv.next().await;
    srv.send_frame(frames::headers(1).response(304).field("content-length", "10")).await;
    srv.send_frame(frames::data(1, "").eos()).await;
    let _ = timeout(Duration::from_millis(400), task).await;
    drain("q23", &mut srv, 100).await;
}

#[tokio::test]
async fn q25_data_after_goaway_uncounted() {
    let (io, mut cli) = mock::new();
    let task = tokio::spawn(async move {
        let mut conn = server::handshake(io).await.unwrap();
        let (q, respond) = conn.accept().await.unwrap().unwrap();
        conn.graceful_shutdown();
        let mut body = q.into_body();
        let a = async move { while let Some(d) = body.data().await { match d { Ok(d) => { println!("  stream 1 read {}", d.len()); let n = d.len(); body.flow_control().release_capacity(n).unwrap(); } Err(e) => { println!("  body err {e}"); break } } } let _k = respond; };
        let b = async { while let Some(r) = conn.accept().await { match r { Ok((q, _)) => println!("  accepted {:?}", q.uri()), Err(e) => { println!("  accept err {e}"); break } } } println!("  conn done"); };
        tokio::join!(a, b);
    });
    cli.assert_server_handshake().await;
    cli.send_bytes(&raw(1, 4, 1, &get_req())).await;
    // expect GOAWAY(MAX) + ping
    for _ in 0..2 { if let Ok(Some(Ok(f))) = timeout(Duration::from_millis(200), cli.next()).await { println!("  peer saw {:?}", f); } }
    cli.send_bytes(&raw(6, 1, 0, &[0x0b, 0x7b, 0xa2, 0xf0, 0x8b, 0x9b, 0xfe, 0x54])).await;
    if let Ok(Some(Ok(f))) = timeout(Duration::from_millis(200), cli.next()).await { println!("  peer saw {:?}", f); }
    // stream 3 was already in flight: its DATA consumes connection window
    let chunk = vec![b'x'; 16383];
    cli.send_bytes(&raw(1, 4, 3, &get_req())).await;
    for _ in 0..4 { cli.send_bytes(&raw(0, 0, 3, &chunk)).await; }
    cli.send_bytes(&raw(0, 0, 3, &vec![b'x'; 3])).await;
    println!("  sent 65535 on ignored stream 3; client connection window now 0");
    drain("q25", &mut cli, 400).await;
    println!("  (no conn WINDOW_UPDATE above => stream 1 can never send its body)");
    task.abort();
}

#[tokio::test]
async fn q27_push_on_closed_parent() {
    let (io, mut cli) = mock::new();
    let task = tokio::spawn(async move {
        let mut conn = server::handshake(io).await.unwrap();
        let (_q, mut respond) = conn.accept().await.unwrap().unwrap();
        let _s = respond.send_response(Response::new(()), true).unwrap();
        // flush
        let _ = timeout(Duration::from_millis(50), conn.accept()).await;
        let r = respond.push_request(push_req());
        println!("  push_request after END_STREAM -> {:?}", r.as_ref().map(|_| "Ok").map_err(|e| e.to_string()));
        let _ = conn.accept().await;
        drop(r);
    });
    cli.assert_server_handshake().await;
    cli.send_bytes(&raw(1, 5, 1, &get_req())).await;
    drain("q27", &mut cli, 200).await;
    task.abort();
}

#[tokio::test]
async fn q28_client_goaway_kills_server_streams() {
    let (io, mut cli) = mock::new();
    let task = tokio::spawn(async move {
        let mut conn = server::handshake(io).await.unwrap();
        let (_q, mut respond) = conn.accept().await.unwrap().unwrap();
        let b = tokio::spawn(async move { let r = conn.accept().await; println!("  accept {:?}", r.map(|x| x.map(|_| ()).map_err(|e| e.to_string()))); });
        sleep(Duration::from_millis(100)).await;
        let r = respond.send_response(Response::new(()), true);
        println!("  send_response after client GOAWAY(last=0,NO_ERROR) -> {:?}", r.map(|_| "Ok").map_err(|e| e.to_string()));
        let _ = b.await;
    });
    cli.assert_server_handshake().await;
    cli.send_bytes(&raw(1, 4, 1, &get_req())).await;
    sleep(Duration::from_millis(30)).await;
    // client says: I processed no server-initiated streams, shutting down gracefully
    cli.send_bytes(&raw(7, 0, 0, &[0; 8])).await;
    drain("q28", &mut cli, 300).await;
    sleep(Duration::from_millis(300)).await;
    task.abort();
}

#[tokio::test]
async fn q28b_server_goaway_kills_pushed() {
    let (io, mut srv) = mock::new();
    let task = tokio::spawn(async move {
        let (mut c, h2) = client::handshake(io).await.unwrap();
        let (mut resp, _s) = c.send_request(get(), false).unwrap();
        let mut pp = resp.push_promises();
        let conn = tokio::spawn(async move { println!("  conn {:?}", h2.await) });
        let x = pp.push_promise().await.unwrap().unwrap();
        let (rq, pr) = x.into_parts();
        println!("  promised {:?}", rq.uri());
        println!("  pushed resp -> {:?}", pr.await.map(|r| r.status()).map_err(|e| e.to_string()));
        println!("  main resp -> {:?}", resp.await.map(|r| r.status()).map_err(|e| e.to_string()));
        drop(conn);
    });
    srv.assert_client_handshake().await;
    let _ = srv.next().await;
    srv.send_frame(frames::push_promise(1, 2).request("GET", "https://a.b/x")).await;
    sleep(Duration::from_millis(30)).await;
    srv.send_bytes(&raw(7, 0, 0, &[0,0,0,1, 0,0,0,0])).await; // GOAWAY(last client stream = 1, NO_ERROR)
    sleep(Duration::from_millis(30)).await;
    srv.send_frame(frames::headers(2).response(200).eos()).await;
    srv.send_frame(frames::headers(1).response(200).eos()).await;
    let _ = timeout(Duration::from_millis(500), task).await;
}

#[tokio::test]
async fn q30_send_trailers_unchecked() {
    let (io, mut srv) = mock::new();
    let task = tokio::spawn(async move {
        let (mut c, h2) = client::handshake(io).await.unwrap();
        let (_resp, mut s) = c.send_request(get(), false).unwrap();
        let conn = tokio::spawn(async move { let _ = h2.await; });
        let mut t = HeaderMap::new();
        t.insert("connection", "close".parse().unwrap());
        println!("  send_trailers(connection: close) -> {:?}", s.send_trailers(t).map_err(|e| e.to_string()));
        sleep(Duration::from_millis(100)).await;
        drop(conn); drop(s); drop(c);
    });
    srv.assert_client_handshake().await;
    drain("q30", &mut srv, 150).await;
    task.abort();
}

// Writer that always reports Ok(0).
struct ZeroW;
impl AsyncRead for ZeroW { fn poll_read(self: std::pin::Pin<&mut Self>, _: &mut std::task::Context<'_>, _: &mut tokio::io::ReadBuf<'_>) -> std::task::Poll<std::io::Result<()>> { std::task::Poll::Pending } }
impl AsyncWrite for ZeroW {
    fn poll_write(self: std::pin::Pin<&mut Self>, _: &mut std::task::Context<'_>, _: &[u8]) -> std::task::Poll<std::io::Result<usize>> { std::task::Poll::Ready(Ok(0)) }
    fn poll_flush(self: std::pin::Pin<&mut Self>, _: &mut std::task::Context<'_>) -> std::task::Poll<std::io::Result<()>> { std::task::Poll::Ready(Ok(())) }
    fn poll_shutdown(self: std::pin::Pin<&mut Self>, _: &mut std::task::Context<'_>) -> std::task::Poll<std::io::Result<()>> { std::task::Poll::Ready(Ok(())) }
}

#[test]
fn q20_ok0_writer_spins() {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let r = rt.block_on(async {
            // server handshake flushes SETTINGS first
            let h = server::handshake(ZeroW);
            tokio::time::timeout(Duration::from_millis(200), h).await.map(|x| x.map(|_| ()).map_err(|e| e.to_string()))
        });
        let _ = tx.send(format!("{:?}", r));
    });
    match rx.recv_timeout(Duration::from_secs(5)) {
        Ok(s) => println!("  handshake returned: {s}"),
        Err(_) => println!("  still spinning after 5s; tokio timeout(200ms) never fired because the task never yields"),
    }
}

#[tokio::test]
async fn q15b_control_drop_all_refs() {
    let (io, mut cli) = mock::new();
    let task = tokio::spawn(async move {
        let mut conn = server::handshake(io).await.unwrap();
        let (req, respond) = conn.accept().await.unwrap().unwrap();
        let body = req.into_body();
        let a = async { sleep(Duration::from_millis(100)).await; drop(body); drop(respond); println!("  server dropped body AND SendResponse"); sleep(Duration::from_secs(5)).await; };
        let b = async { while let Some(r) = conn.accept().await { if r.is_err() { break } } };
        tokio::join!(a, b);
    });
    cli.assert_server_handshake().await;
    cli.send_bytes(&raw(1, 4, 1, &get_req())).await;
    let chunk = vec![b'x'; 16383];
    for _ in 0..4 { cli.send_bytes(&raw(0, 0, 1, &chunk)).await; }
    cli.send_bytes(&raw(0, 0, 1, &vec![b'x'; 3])).await;
    drain("q15b", &mut cli, 500).await;
    task.abort();
}

#[tokio::test]
async fn q31_parent_reset_orphans_push() {
    let (io, mut cli) = mock::new();
    let task = tokio::spawn(async move {
        let mut conn = server::handshake(io).await.unwrap();
        let (_q, mut respond) = conn.accept().await.unwrap().unwrap();
        let c = tokio::spawn(async move { let r = futures::future::poll_fn(|cx| conn.poll_closed(cx)).await; println!("  conn closed {:?}", r.map_err(|e| e.to_string())); });
        let mut s = respond.send_response(Response::new(()), false).unwrap();
        s.send_data(Bytes::from(vec![0u8; 100_000]), false).unwrap();
        let mut pushed = respond.push_request(push_req()).unwrap();
        sleep(Duration::from_millis(150)).await; // client resets parent meanwhile
        let r = pushed.send_response(Response::new(()), true);
        println!("  pushed.send_response -> {:?}", r.as_ref().map(|_| "Ok").map_err(|e| e.to_string()));
        let w = timeout(Duration::from_millis(400), futures::future::poll_fn(|cx| pushed.poll_reset(cx))).await;
        println!("  pushed.poll_reset -> {:?}", w.map(|x| x.map_err(|e| e.to_string())).map_err(|_| "TIMED OUT"));
        drop(r);
        c.abort();
    });
    cli.assert_server_handshake().await;
    cli.send_bytes(&raw(1, 5, 1, &get_req())).await;
    drain("q31", &mut cli, 80).await;
    cli.send_bytes(&raw(3, 0, 1, &[0, 0, 0, 8])).await;
    drain("q31-after-rst", &mut cli, 700).await;
    let _ = task.await;
}

#[tokio::test]
async fn q32_max_header_list() {
    for (name, l) in [("list size 99 (below limit 100)", 24usize), ("list size 100 (== limit)", 25), ("trailers oversize", 0)] {
        let (io, mut srv) = mock::new();
        let task = tokio::spawn(async move {
            let (mut c, h2) = client::Builder::new().max_header_list_size(100).handshake::<_, Bytes>(io).await.unwrap();
            let (resp, _s) = c.send_request(get(), true).unwrap();
            let conn = tokio::spawn(async move { let _ = h2.await; });
            match resp.await {
                Ok(r) => { let (p, mut b) = r.into_parts(); println!("  [{name}] response {:?} hdrs={:?}", p.status, p.headers.len());
                    let t = b.trailers().await; println!("  [{name}] trailers {:?}", t.map(|m| m.map(|m| m.keys().map(|k| k.as_str().to_string()).collect::<Vec<_>>())).map_err(|e| e.to_string())); }
                Err(e) => println!("  [{name}] response err {e}"),
            }
            drop(conn); drop(c);
        });
        srv.assert_client_handshake().await;
        let _ = srv.next().await;
        if l > 0 {
            srv.send_frame(frames::headers(1).response(200).field("x", "v".repeat(l)).eos()).await;
        } else {
            srv.send_frame(frames::headers(1).response(200)).await;
            srv.send_frame(frames::headers(1).field("a", "1").field("b", "v".repeat(200)).field("c", "3").eos()).await;
        }
        let _ = timeout(Duration::from_millis(300), task).await;
    }
}

#[tokio::test]
async fn q33_e2e_reset_before_open() {
    let (a, b) = tokio::io::duplex(1 << 16);
    let srv = tokio::spawn(async move {
        let mut conn = server::handshake(b).await.unwrap();
        let mut keep = vec![];
        while let Some(r) = conn.accept().await {
            match r { Ok(x) => { println!("  server got {:?}", x.0.uri()); keep.push(x) } Err(e) => { println!("  server accept err: {e}"); break } }
        }
    });
    let (mut c, h2) = client::handshake(a).await.unwrap();
    let conn = tokio::spawn(async move { println!("  client conn {:?}", h2.await.map_err(|e| e.to_string())) });
    let (r0, mut s0) = c.send_request(get(), false).unwrap();
    let (r1, _s1) = c.send_request(get(), false).unwrap();
    // cancel stream 1 before the connection task ran
    s0.send_reset(Reason::CANCEL);
    println!("  other request -> {:?}", timeout(Duration::from_millis(500), r1).await.map(|x| x.map(|r| r.status()).map_err(|e| e.to_string())).map_err(|_| "pending"));
    drop((r0, s0, c));
    srv.abort(); conn.abort();
}

#[tokio::test]
async fn q34_self_dep_hpack_desync() {
    // Control: same bytes, stream 1 has a valid priority dependency.
    for (name, dep) in [("control dep=0", 0u8), ("self-dependency", 1u8)] {
        println!("--- q34 {name}");
        let (io, mut cli) = mock::new();
        let task = tokio::spawn(async move {
            let mut conn = server::handshake(io).await.unwrap();
            let mut keep = vec![];
            while let Some(r) = conn.accept().await {
                match r { Ok((q, s)) => { println!("  accepted {:?} {:?}", q.uri(), q.headers()); keep.push((q, s)); } Err(e) => { println!("  accept err {e}"); break } }
            }
        });
        cli.assert_server_handshake().await;
        // PRIORITY flag: 4 byte dep + weight, then literal-with-incremental-indexing "x-a: b"
        let mut p = vec![0, 0, 0, dep, 16];
        p.extend(get_req());
        p.extend_from_slice(&[0x40, 3, b'x', b'-', b'a', 1, b'b']);
        cli.send_bytes(&raw(1, 0x24 | 1, 1, &p)).await;
        // next request refers to dynamic entry 62
        let mut q = get_req(); q.push(0x80 | 62);
        cli.send_bytes(&raw(1, 5, 3, &q)).await;
        drain("q34", &mut cli, 150).await;
        task.abort();
    }
}

#[tokio::test]
async fn q35_pp_empty_first_fragment() {
    let (io, mut srv) = mock::new();
    let task = tokio::spawn(async move {
        let (mut c, h2) = client::handshake(io).await.unwrap();
        let (mut resp, _s) = c.send_request(get(), false).unwrap();
        let mut pp = resp.push_promises();
        let conn = tokio::spawn(async move { println!("  conn {:?}", h2.await.map_err(|e| e.to_string())) });
        println!("  push {:?}", pp.push_promise().await.map(|x| x.map(|y| y.request().uri().clone()).map_err(|e| e.to_string())));
        drop(conn);
    });
    srv.assert_client_handshake().await;
    let _ = srv.next().await;
    srv.send_bytes(&raw(5, 0, 1, &[0, 0, 0, 2])).await;
    srv.send_bytes(&raw(9, 4, 1, &get_req())).await;
    let _ = timeout(Duration::from_millis(300), task).await;
    drain("q35", &mut srv, 100).await;
}

async fn counter_case(leak: bool) {
    println!("--- q36 leak_setup={leak}");
    let (io, mut srv) = mock::new();
    let (go_tx, mut go_rx) = tokio::sync::mpsc::unbounded_channel::<u8>();
    let task = tokio::spawn(async move {
        let (mut c, h2) = client::Builder::new().max_concurrent_reset_streams(1).reset_stream_duration(Duration::from_millis(200)).handshake::<_, Bytes>(io).await.unwrap();
        let conn = tokio::spawn(async move { println!("  conn(leak={leak}) {:?}", h2.await.map_err(|e| e.to_string())) });
        go_rx.recv().await; // settings applied
        let a = c.clone().send_request(get(), false).unwrap();
        go_rx.recv().await;
        if leak {
            let b = c.clone().send_request(get(), false).unwrap(); // stream 3: pending_open
            println!("  client: stream 3 is queued; dropping its handles");
            tokio::task::yield_now().await;
            drop(b); // scheduled reset while still pending_open
        }
        go_rx.recv().await;
        let d = c.clone().send_request(get(), false).unwrap();
        println!("  client: opened cancel-target stream");
        go_rx.recv().await;
        println!("  client: dropping cancel-target handles");
        drop(d);
        go_rx.recv().await;
        drop(a); drop(conn); drop(c);
    });
    srv.assert_client_handshake_with_settings(frames::settings().max_concurrent_streams(1)).await;
    sleep(Duration::from_millis(20)).await;
    go_tx.send(0).unwrap(); // -> send a
    let _ = srv.next().await; // HEADERS(1)
    go_tx.send(0).unwrap(); // -> leak setup
    sleep(Duration::from_millis(30)).await;
    if leak {
        sleep(Duration::from_millis(300)).await;
        srv.send_bytes(&raw(6, 0, 0, &[7; 8])).await; // wakes conn -> expiry pass
        let _ = srv.next().await;
    }
    srv.send_bytes(&raw(1, 5, 1, &[0x88])).await; // finish stream 1 half
    srv.send_bytes(&raw(3, 0, 1, &[0, 0, 0, 0])).await; // and reset it so a slot frees
    go_tx.send(0).unwrap(); // -> open target
    let want: u32 = if leak { 5 } else { 3 };
    loop {
        match timeout(Duration::from_millis(400), srv.next()).await {
            Ok(Some(Ok(f))) => { println!("  peer saw {:?}", f); if let frame::Frame::Headers(h) = f { if u32::from(h.stream_id()) == want { break } } }
            o => { println!("  (no more) {}", o.is_ok()); break }
        }
    }
    go_tx.send(0).unwrap(); // -> drop target
    match timeout(Duration::from_millis(100), srv.next()).await { Ok(Some(Ok(f))) => println!("  peer saw {:?}", f), _ => println!("  no RST seen") }
    println!("  peer: late HEADERS+DATA on just-cancelled stream {want} (inside 200ms remember window)");
    srv.send_bytes(&raw(1, 4, want, &[0x88])).await;
    srv.send_bytes(&raw(0, 0, want, b"late")).await;
    drain("q36-late", &mut srv, 80).await;
    task.abort();
}

#[tokio::test]
async fn q36_reset_counter_leak() {
    counter_case(false).await;
    sleep(Duration::from_millis(50)).await;
    counter_case(true).await;
}

#[tokio::test]
async fn q38_shared_send_waker() { q38(true).await }
#[tokio::test]
async fn q38_control_no_poll_reset() { q38(false).await }
async fn q38(with_body: bool) {
    use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
    let (io, mut srv) = mock::new();
    let (go_tx, mut go_rx) = tokio::sync::mpsc::unbounded_channel::<u8>();
    let task = tokio::spawn(async move {
        let (mut c, h2) = client::handshake(io).await.unwrap();
        let conn = tokio::spawn(async move { let _ = h2.await; });
        go_rx.recv().await;
        let first = c.send_request(get(), true).unwrap();
        go_rx.recv().await;
        let (r2, mut s2) = c.send_request(get(), false).unwrap(); // queued; SendRequest now not ready
        let mut c9 = c.clone();
        let ready_fut = tokio::spawn(async move {
            let r = c.ready().await;
            println!("  SendRequest::ready() -> {}", r.is_ok());
        });
        tokio::task::yield_now().await;
        // Body task registers on the same stream afterwards, as a body pipe does.
        let body = tokio::spawn(async move {
            if !with_body { sleep(Duration::from_secs(3)).await; }
            let r = futures::future::poll_fn(|cx| s2.poll_reset(cx)).await;
            println!("  poll_reset woke -> {:?}", r.map_err(|e| e.to_string()));
            sleep(Duration::from_secs(3)).await;
            drop(s2);
        });
        tokio::task::yield_now().await;
        go_rx.recv().await;
        let w = timeout(Duration::from_millis(500), ready_fut).await;
        println!("  ready task finished? {}", w.is_ok());
        body.abort(); drop(first); drop(r2); drop(conn);
    });
    srv.assert_client_handshake_with_settings(frames::settings().max_concurrent_streams(1)).await;
    sleep(Duration::from_millis(20)).await;
    go_tx.send(0).unwrap();
    let _ = srv.next().await;
    go_tx.send(0).unwrap();
    sleep(Duration::from_millis(50)).await;
    srv.send_bytes(&raw(1, 5, 1, &[0x88])).await; // stream 1 done; stream 3 may open
    let f = timeout(Duration::from_millis(100), srv.next()).await;
    println!("  peer saw {:?}", f.map(|x| x.map(|y| y.map(|z| format!("{:?}", z)).map_err(|_| ()))));
    go_tx.send(0).unwrap();
    let _ = timeout(Duration::from_secs(2), task).await;
}

#[tokio::test]
async fn q37_userinfo_authority() {
    let (io, mut srv) = mock::new();
    let task = tokio::spawn(async move {
        let (mut c, h2) = client::handshake(io).await.unwrap();
        let r = Request::builder().uri("https://user:secret@a.b/").body(()).unwrap();
        let (_resp, _s) = c.send_request(r, true).unwrap();
        let _ = h2.await;
    });
    srv.assert_client_handshake().await;
    if let Some(Ok(frame::Frame::Headers(h))) = srv.next().await {
        println!("  :authority on the wire = {:?}", h.into_parts().0.authority);
    }
    task.abort();
}

#[tokio::test]
async fn q39_pending_open_hoards_capacity() {
    let (io, mut srv) = mock::new();
    let (go_tx, mut go_rx) = tokio::sync::mpsc::unbounded_channel::<u8>();
    let task = tokio::spawn(async move {
        let (mut c, h2) = client::handshake(io).await.unwrap();
        let conn = tokio::spawn(async move { let _ = h2.await; });
        go_rx.recv().await;
        let (_r1, mut s1) = c.clone().send_request(get(), false).unwrap();
        go_rx.recv().await;
        let (_r3, mut s3) = c.clone().send_request(get(), false).unwrap(); // over the limit: never opened
        s3.reserve_capacity(65535);
        tokio::task::yield_now().await;
        println!("  queued (unopened) stream 3 capacity = {}", s3.capacity());
        s1.reserve_capacity(10);
        let w = timeout(Duration::from_millis(500), futures::future::poll_fn(|cx| s1.poll_capacity(cx))).await;
        println!("  open stream 1 capacity after reserve_capacity(10) = {} ; poll_capacity -> {:?}", s1.capacity(), w.map(|x| x.map(|y| y.map_err(|e| e.to_string()))).map_err(|_| "TIMED OUT"));
        drop(conn);
    });
    srv.assert_client_handshake_with_settings(frames::settings().max_concurrent_streams(1)).await;
    sleep(Duration::from_millis(20)).await;
    go_tx.send(0).unwrap();
    let _ = srv.next().await;
    go_tx.send(0).unwrap();
    let _ = timeout(Duration::from_secs(1), task).await;
    drain("q39", &mut srv, 50).await;
}

#[tokio::test]
async fn q41_cancelled_queued_stream_eats_conn_window() {
    let (io, mut srv) = mock::new();
    let (go_tx, mut go_rx) = tokio::sync::mpsc::unbounded_channel::<u8>();
    let task = tokio::spawn(async move {
        let (mut c, h2) = client::handshake(io).await.unwrap();
        let conn = tokio::spawn(async move { println!("  conn ended {:?}", h2.await.map_err(|e| e.to_string())); });
        go_rx.recv().await;
        let r1 = c.clone().send_request(get(), false).unwrap();
        go_rx.recv().await;
        let mut r3 = c.clone().send_request(get(), false).unwrap(); // queued
        r3.1.reserve_capacity(65535);
        tokio::task::yield_now().await;
        drop(r3); // cancelled while queued
        go_rx.recv().await; // stream 1 finished by peer, stream 3 popped from the open queue
        sleep(Duration::from_millis(50)).await;
        let (_r5, mut s5) = c.clone().send_request(get(), false).unwrap();
        s5.send_data(Bytes::from_static(b"hello"), false).unwrap();
        s5.reserve_capacity(5);
        sleep(Duration::from_millis(300)).await;
        println!("  new stream capacity = {}", s5.capacity());
        drop(r1);
        go_rx.recv().await;
        drop(s5); drop(conn);
    });
    srv.assert_client_handshake_with_settings(frames::settings().max_concurrent_streams(1)).await;
    sleep(Duration::from_millis(20)).await;
    go_tx.send(0).unwrap();
    let _ = srv.next().await;
    go_tx.send(0).unwrap();
    sleep(Duration::from_millis(30)).await;
    srv.send_bytes(&raw(1, 5, 1, &[0x88])).await;
    srv.send_bytes(&raw(3, 0, 1, &[0, 0, 0, 0])).await;
    go_tx.send(0).unwrap();
    println!("  peer stream: frames below; the 5 byte DATA on stream 5 should appear");
    drain("q41", &mut srv, 500).await;
    go_tx.send(0).unwrap();
    let _ = timeout(Duration::from_millis(100), task).await;
}

#[tokio::test]
async fn q42_misc() {
    // leading/trailing whitespace in field value on server
    println!("--- value whitespace");
    let (io, mut cli) = mock::new();
    let t = tokio::spawn(async move {
        let mut conn = server::handshake(io).await.unwrap();
        let (q, _r) = conn.accept().await.unwrap().unwrap();
        println!("  accepted value = {:?}", q.headers().get("x"));
    });
    cli.assert_server_handshake().await;
    let mut h = get_req(); h.extend(lit("x", " padded "));
    cli.send_bytes(&raw(1, 5, 1, &h)).await;
    let _ = timeout(Duration::from_millis(100), t).await;

    println!("--- 101 response on client");
    let (io, mut srv) = mock::new();
    let t = tokio::spawn(async move {
        let (mut c, h2) = client::handshake(io).await.unwrap();
        let (resp, _s) = c.send_request(get(), false).unwrap();
        let conn = tokio::spawn(async move { let _ = h2.await; });
        println!("  response future -> {:?}", timeout(Duration::from_millis(200), resp).await.map(|x| x.map(|r| r.status()).map_err(|e| e.to_string())).map_err(|_| "TIMED OUT (treated as informational)"));
        drop(conn);
    });
    srv.assert_client_handshake().await;
    let _ = srv.next().await;
    srv.send_frame(frames::headers(1).response(101)).await;
    let _ = timeout(Duration::from_millis(400), t).await;
    drain("101", &mut srv, 30).await;

    println!("--- te multi value on send");
    let (io, mut srv) = mock::new();
    let t = tokio::spawn(async move {
        let (mut c, h2) = client::handshake(io).await.unwrap();
        let r = Request::builder().uri("https://a.b/").header("te", "trailers").header("te", "gzip").body(()).unwrap();
        println!("  send_request -> {:?}", c.send_request(r, true).map(|_| "Ok").map_err(|e| e.to_string()));
        let _ = h2.await;
    });
    srv.assert_client_handshake().await;
    drain("te", &mut srv, 60).await;
    t.abort();
}

#[tokio::test]
async fn q13b_trailers_after_default_expiry() {
    let (io, mut cli) = mock::new();
    let task = tokio::spawn(async move {
        let mut conn = server::handshake(io).await.unwrap();
        let (a, mut respond) = conn.accept().await.unwrap().unwrap();
        respond.send_reset(Reason::CANCEL);
        drop(a); drop(respond);
        let r = conn.accept().await;
        println!("  accept {:?}", r.map(|x| x.map(|_| ()).map_err(|e| e.to_string())));
    });
    cli.assert_server_handshake().await;
    cli.send_bytes(&raw(1, 4, 1, &get_req())).await;
    drain("q13b", &mut cli, 50).await;
    sleep(Duration::from_millis(1100)).await;
    cli.send_bytes(&raw(6, 0, 0, &[1; 8])).await; // connection polled, reset stream expired
    drain("q13b", &mut cli, 50).await;
    cli.send_bytes(&raw(1, 5, 1, &lit("t", "v"))).await;
    drain("q13b", &mut cli, 100).await;
    let _ = timeout(Duration::from_millis(100), task).await;
}
