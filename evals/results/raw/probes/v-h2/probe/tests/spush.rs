use probe::*;
fn pr() -> Request<()> { req_get("https://a.test/x") }
async fn run_case<F, Fut>(name: &str, b: server::Builder, input: Vec<u8>, ms: u64, app: F)
where F: FnOnce(Request<h2::RecvStream>, server::SendResponse<Bytes>, Log) -> Fut + Send + 'static, Fut: std::future::Future<Output=()> + Send + 'static {
    let (io, mut c) = mock::new();
    let l = log();
    let l2 = l.clone();
    let j = tokio::spawn(async move {
        let mut conn = b.handshake::<_, Bytes>(io).await.unwrap();
        let (rq, sr) = conn.accept().await.unwrap().unwrap();
        tokio::spawn(app(rq, sr, l2.clone()));
        while let Some(r) = conn.accept().await { lg(&l2, format!("accept {:?}", r.map(|_| ()))); }
        lg(&l2, format!("wired at end {}", conn.num_wired_streams()));
    });
    c.assert_server_handshake().await;
    c.send_bytes(&input).await;
    let fr = dump(&mut c, ms).await;
    sleep(Duration::from_millis(20)).await;
    if j.is_finished() { if let Err(e) = j.await { if e.is_panic() { lg(&l, "CONN PANICKED".into()); } } }
    show(name, &fr, &l);
}
fn hreq() -> Vec<u8> { f(HEADERS, EH | ES, 1, &req(&[])) }
async fn drop_n(n: usize) {
    run_case(&format!("drop {n} pushes"), server::Builder::new(), hreq(), 150, move |_rq, mut sr, l| async move {
        for _ in 0..n {
            match sr.push_request(pr()) { Ok(p) => drop(p), Err(e) => { lg(&l, format!("push err {e:?}")); break; } }
        }
        let _ = sr.send_response(Response::new(()), false);
        sleep(Duration::from_millis(200)).await;
    }).await;
}
#[tokio::test]
async fn drop_1() { drop_n(1).await; }
#[tokio::test]
async fn drop_20() { drop_n(20).await; }
#[tokio::test]
async fn drop_60() { drop_n(60).await; }
#[tokio::test]
async fn drop_reset0() {
    let mut b = server::Builder::new();
    b.max_concurrent_reset_streams(0);
    run_case("drop 1 reset limit 0", b, hreq(), 150, |_rq, mut sr, _l| async move {
        drop(sr.push_request(pr()).unwrap());
        let _ = sr.send_response(Response::new(()), false);
        sleep(Duration::from_millis(200)).await;
    }).await;
}
#[tokio::test]
async fn parent_reset_orphan() {
    run_case("parent reset orphan", server::Builder::new(), hreq(), 300, |_rq, mut sr, l| async move {
        let mut p = sr.push_request(pr()).unwrap();
        sr.send_reset(Reason::CANCEL);
        let r = p.send_response(Response::new(()), false);
        lg(&l, format!("pushed send_response {:?}", r.as_ref().map(|_| ())));
        if let Ok(mut st) = r {
            lg(&l, format!("send_data {:?}", st.send_data(Bytes::from_static(b"hi"), true)));
            let pr = timeout(Duration::from_millis(200), poll_fn(|cx| p.poll_reset(cx))).await;
            lg(&l, format!("poll_reset {pr:?}"));
        }
    }).await;
}
#[tokio::test]
async fn failed_push_leak() {
    run_case("failed pushes", server::Builder::new(), hreq(), 150, |_rq, mut sr, l| async move {
        let bad = || Request::builder().method("POST").uri("https://a.test/x").body(()).unwrap();
        let mut errs = 0;
        for _ in 0..100 { if sr.push_request(bad()).is_err() { errs += 1; } }
        lg(&l, format!("errors {errs}"));
        let _ = sr.send_response(Response::new(()), true);
    }).await;
    // show wired count
    let (io, mut c) = mock::new();
    let j = tokio::spawn(async move {
        let mut conn = server::handshake(io).await.unwrap();
        let (_rq, mut sr) = conn.accept().await.unwrap().unwrap();
        println!("wired before {}", conn.num_wired_streams());
        let bad = || Request::builder().method("POST").uri("https://a.test/x").body(()).unwrap();
        for _ in 0..100 { let _ = sr.push_request(bad()); }
        println!("wired after 100 failed {}", conn.num_wired_streams());
        let _ = sr.send_response(Response::new(()), true);
        drop(sr); drop(_rq);
        tokio::select! { _ = conn.accept() => {}, _ = sleep(Duration::from_millis(100)) => {} }
        println!("wired after parent done {}", conn.num_wired_streams());
    });
    c.assert_server_handshake().await;
    c.send_bytes(&hreq()).await;
    let _ = dump(&mut c, 200).await;
    let _ = j.await;
}
#[tokio::test]
async fn push_after_close() {
    run_case("push after END_STREAM", server::Builder::new(), hreq(), 150, |_rq, mut sr, l| async move {
        let _ = sr.send_response(Response::new(()), true);
        sleep(Duration::from_millis(50)).await;
        let r = sr.push_request(pr());
        lg(&l, format!("push_request {:?}", r.as_ref().map(|_| ())));
        sleep(Duration::from_millis(100)).await;
        drop(r);
    }).await;
}
#[tokio::test]
async fn drop_default_flow_blocked() {
    let (io, mut c) = mock::new();
    let j = tokio::spawn(async move {
        let mut conn = server::handshake(io).await.unwrap();
        let (_rq, mut sr) = conn.accept().await.unwrap().unwrap();
        let mut st = sr.send_response(Response::new(()), false).unwrap();
        st.send_data(Bytes::from_static(b"blocked"), false).unwrap();
        drop(sr.push_request(pr()).unwrap());
        println!("pushed handle dropped");
        let _keep = (st, sr, _rq);
        while let Some(_) = conn.accept().await {}
    });
    let mut s = h2::frame::Settings::default();
    s.set_initial_window_size(Some(0));
    c.assert_server_handshake_with_settings(s).await;
    c.send_bytes(&hreq()).await;
    let a = dump(&mut c, 1300).await;
    println!("before WU: {a:?}");
    c.send_bytes(&wu(1, 100)).await;
    let b = dump(&mut c, 100).await;
    println!("after WU: {b:?}");
    println!("conn: {:?}", j.await.map_err(|e| e.is_panic()));
}
