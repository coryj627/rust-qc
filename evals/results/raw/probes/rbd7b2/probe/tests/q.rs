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
#[tokio::test]
async fn q01_dropped_body_retains_conn_window() {
    let (io, mut c) = mock::new();
    let cl = async move {
        c.assert_server_handshake().await;
        let mut h = base(); h[0] = 0x83;
        c.send_bytes(&fr(1, 4, 1, &h)).await;
        let d = vec![b'z'; 16384];
        for _ in 0..3 { c.send_bytes(&fr(0, 0, 1, &d)).await; }
        drain(&mut c, 300, "q01").await;
    };
    let s = async move {
        let mut srv = server::handshake(io).await.unwrap();
        let (req, mut resp) = srv.accept().await.unwrap().unwrap();
        let _send = resp.send_response(Response::new(()), false).unwrap();
        idle_ms(20).await;
        let acc = async { tokio::time::timeout(Duration::from_millis(50), srv.accept()).await.ok(); };
        join(idle_ms(30), acc).await;
        drop(req); // handler ignores the request body but keeps streaming its response
        println!("[q01] request body dropped, response stream kept open");
        tokio::time::timeout(Duration::from_millis(600), srv.accept()).await.ok();
    };
    tokio::time::timeout(Duration::from_secs(5), join(cl, s)).await.ok();
}

#[tokio::test]
async fn q02_push_dropped_no_rst_long() {
    let (io, mut c) = mock::new();
    let cl = async move {
        c.assert_server_handshake().await;
        c.send_bytes(&fr(1, 5, 1, &base())).await;
        drain(&mut c, 700, "q02").await;
    };
    let s = async move {
        let mut srv = server::handshake(io).await.unwrap();
        let (_req, mut resp) = srv.accept().await.unwrap().unwrap();
        let mut _st = resp.send_response(Response::new(()), false).unwrap();
        let p = resp.push_request(Request::builder().uri("https://x.com/p").body(()).unwrap()).unwrap();
        drop(p);
        tokio::time::timeout(Duration::from_millis(1000), srv.accept()).await.ok();
    };
    tokio::time::timeout(Duration::from_secs(5), join(cl, s)).await.ok();
}

#[tokio::test]
async fn q03_push_dropped_over_reset_limit() {
    let (io, mut c) = mock::new();
    let cl = async move {
        c.assert_server_handshake().await;
        c.send_bytes(&fr(1, 5, 1, &base())).await;
        drain(&mut c, 200, "q03").await;
    };
    let s = async move {
        let mut srv = server::handshake(io).await.unwrap();
        let (_req, mut resp) = srv.accept().await.unwrap().unwrap();
        let mut st = resp.send_response(Response::new(()), false).unwrap();
        for _ in 0..60 {
            let p = resp.push_request(Request::builder().uri("https://x.com/p").body(()).unwrap()).unwrap();
            drop(p);
        }
        st.send_data(Bytes::new(), true).unwrap();
        tokio::time::timeout(Duration::from_millis(150), srv.accept()).await.ok();
        println!("[q03] server survived");
    };
    tokio::time::timeout(Duration::from_secs(5), join(cl, s)).await.ok();
}

#[tokio::test]
async fn q04_goaway_data_window() {
    let (io, mut c) = mock::new();
    let cl = async move {
        c.assert_server_handshake().await;
        let mut h = base(); h[0] = 0x83;
        c.send_bytes(&fr(1, 4, 1, &h)).await;
        idle_ms(50).await;
        c.send_bytes(&fr(6, 1, 0, &[0x0b, 0x7b, 0xa2, 0xf0, 0x8b, 0x9b, 0xfe, 0x54])).await;
        idle_ms(30).await;
        c.send_bytes(&fr(1, 4, 3, &h)).await; // raced stream above last_stream_id
        let d = vec![b'z'; 16384];
        for _ in 0..3 { c.send_bytes(&fr(0, 0, 3, &d)).await; }
        drain(&mut c, 300, "q04").await;
    };
    let s = async move {
        let mut srv = server::handshake(io).await.unwrap();
        let (_req, _resp) = srv.accept().await.unwrap().unwrap();
        srv.graceful_shutdown();
        tokio::time::timeout(Duration::from_millis(600), srv.accept()).await.ok();
    };
    tokio::time::timeout(Duration::from_secs(5), join(cl, s)).await.ok();
}

#[tokio::test]
async fn q05_rst_then_push_debug_assert() {
    let (io, mut srv) = mock::new();
    let m = async move {
        srv.assert_client_handshake().await;
        srv.recv_frame(frames::headers(1).request("GET", "https://x.com/").eos()).await;
        srv.send_bytes(&fr(3, 0, 1, &[0, 0, 0, 8])).await;
        let mut p = 2u32.to_be_bytes().to_vec(); p.extend(base());
        srv.send_bytes(&fr(5, 4, 1, &p)).await;
        drain(&mut srv, 100, "q05").await;
    };
    let h = async move {
        let (mut client, conn) = client::handshake(io).await.unwrap();
        let req = Request::builder().uri("https://x.com/").body(()).unwrap();
        let (resp, _) = client.send_request(req, true).unwrap();
        println!("[q05] conn result {:?}", conn.await);
        drop(resp);
    };
    tokio::time::timeout(Duration::from_secs(5), join(m, h)).await.ok();
}

#[tokio::test]
async fn q07_client_frames() {
    for (tag, method, uri) in [("q07a", "GET", "example.com"), ("q07b", "CONNECT", "/x"), ("q07c", "OPTIONS", "http://example.com"), ("q07d", "GET", "/rel")] {
        let (io, mut srv) = mock::new();
        let m = async move {
            srv.assert_client_handshake().await;
            match srv.next().await {
                Some(Ok(h2::frame::Frame::Headers(h))) => println!("[{tag}] {:?}", h.into_parts().0),
                o => println!("[{tag}] other {:?}", o),
            }
        };
        let h = async move {
            let (mut client, mut conn) = client::handshake(io).await.unwrap();
            let req = Request::builder().method(method).uri(uri).body(()).unwrap();
            match client.send_request(req, true) {
                Ok((resp, _)) => { tokio::time::timeout(Duration::from_millis(50), conn.drive(resp)).await.ok(); }
                Err(e) => println!("[{tag}] send_request err {:?}", e),
            }
        };
        tokio::time::timeout(Duration::from_secs(5), join(m, h)).await.ok();
    }
}

#[tokio::test]
async fn q08_exact_panic_threshold_small_list_limit() {
    // Find the header count at which append panics and show it applies with a 1 MiB limit too.
    for (tag, count) in [("q08-24000", 24000u32), ("q08-24600", 24600u32)] {
        let (io, mut c) = mock::new();
        let mut blk = base();
        for i in 0..count { blk.extend(lit(&format!("h{:05}", i), "")); }
        let cl = async move {
            c.assert_server_handshake().await;
            idle_ms(5).await;
            let mut it = blk.chunks(16384);
            let first = it.next().unwrap();
            let chunks: Vec<_> = it.collect();
            c.send_bytes(&fr(1, 1, 1, first)).await;
            for (i, ch) in chunks.iter().enumerate() {
                c.send_bytes(&fr(9, if i + 1 == chunks.len() { 4 } else { 0 }, 1, ch)).await;
            }
            drain(&mut c, 100, tag).await;
        };
        let s = async move {
            let mut srv = server::Builder::new().max_header_list_size(1 << 20).handshake::<_, Bytes>(io).await.unwrap();
            let r = tokio::time::timeout(Duration::from_millis(500), srv.accept()).await;
            println!("[{tag}] accept -> {:?}", r.map(|o| o.map(|x| x.map(|(req, _)| req.headers().len()))));
        };
        tokio::time::timeout(Duration::from_secs(5), join(cl, s)).await.ok();
    }
}

#[tokio::test]
async fn q09_server_rst_then_push() {
    let (io, mut c) = mock::new();
    let cl = async move {
        c.assert_server_handshake().await;
        let mut h = base(); h[0] = 0x83;
        c.send_bytes(&fr(1, 4, 1, &h)).await;
        idle_ms(20).await;
        c.send_bytes(&fr(3, 0, 1, &[0, 0, 0, 8])).await;
        let mut p = 2u32.to_be_bytes().to_vec(); p.extend(base());
        c.send_bytes(&fr(5, 4, 1, &p)).await;
        drain(&mut c, 100, "q09").await;
    };
    let s = async move {
        let mut srv = server::handshake(io).await.unwrap();
        let (_req, _resp) = srv.accept().await.unwrap().unwrap();
        let r = tokio::time::timeout(Duration::from_millis(200), srv.accept()).await;
        println!("[q09] accept -> {:?}", r.map(|o| o.map(|x| x.map(|_| ()))));
    };
    tokio::time::timeout(Duration::from_secs(5), join(cl, s)).await.ok();
}

#[tokio::test]
async fn q10_user_reset_then_push() {
    let (io, mut c) = mock::new();
    let cl = async move {
        c.assert_server_handshake().await;
        let mut h = base(); h[0] = 0x83;
        c.send_bytes(&fr(1, 4, 1, &h)).await;
        idle_ms(40).await;
        let mut p = 2u32.to_be_bytes().to_vec(); p.extend(base());
        c.send_bytes(&fr(5, 4, 1, &p)).await;
        drain(&mut c, 100, "q10").await;
    };
    let s = async move {
        let mut srv = server::handshake(io).await.unwrap();
        let (_req, mut resp) = srv.accept().await.unwrap().unwrap();
        resp.send_reset(Reason::CANCEL);
        let r = tokio::time::timeout(Duration::from_millis(200), srv.accept()).await;
        println!("[q10] accept -> {:?}", r.map(|o| o.map(|x| x.map(|_| ()))));
    };
    tokio::time::timeout(Duration::from_secs(5), join(cl, s)).await.ok();
}
