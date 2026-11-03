use probe::*;
fn rq() -> Request<()> {
    req_get("https://a.test/")
}
fn post() -> Request<()> {
    Request::builder().method("POST").uri("https://a.test/").body(()).unwrap()
}
async fn push_seq(name: &str, b: client::Builder, tail: Vec<u8>) {
    let mut x = cli(b).await;
    let (resp, _) = x.c.send_request(rq(), true).unwrap();
    let t = tokio::spawn(async move { let r = resp.await; println!("resp {:?}", r.map(|_| ())); });
    let _ = dump(&mut x.s, 50).await;
    let mut inp = f(HEADERS, EH, 1, &resp_b("200"));
    inp.extend(tail);
    x.s.send_bytes(&inp).await;
    let fr = dump(&mut x.s, 150).await;
    t.abort();
    report(name, &fr, x).await;
}
fn resp_b(s: &str) -> Vec<u8> { resp(s, &[]) }
#[tokio::test]
async fn push_1xx() {
    let tail = [pp(1, 2), f(HEADERS, EH, 2, &resp_b("100")), f(HEADERS, EH, 2, &resp_b("200"))].concat();
    push_seq("pp 100 200", client::Builder::new(), tail).await;
    let tail = [pp(1, 2), f(HEADERS, EH, 2, &resp_b("100")), f(HEADERS, EH, 2, &resp_b("103"))].concat();
    push_seq("pp 100 103", client::Builder::new(), tail).await;
    let tail = [pp(1, 2), f(HEADERS, EH, 2, &resp_b("200"))].concat();
    push_seq("pp 200 control", client::Builder::new(), tail).await;
}
#[tokio::test]
async fn push_over_limit() {
    let tail = [pp(1, 2), pp(1, 4), f(HEADERS, EH, 2, &resp_b("200")), f(HEADERS, EH, 4, &resp_b("200"))].concat();
    let mut b = client::Builder::new();
    b.max_concurrent_streams(1);
    push_seq("over limit", b, tail).await;
}
async fn one_resp(name: &str, b: client::Builder, mk: Vec<u8>, trailers: bool, post_req: bool) {
    let mut x = cli(b).await;
    let (resp, st) = x.c.send_request(if post_req { post() } else { rq() }, !post_req).unwrap();
    let l = x.l.clone();
    let t = tokio::spawn(async move {
        let _st = st;
        match timeout(Duration::from_millis(400), resp).await {
            Err(_) => lg(&l, "resp TIMEOUT".into()),
            Ok(Err(e)) => lg(&l, format!("resp err {e:?}")),
            Ok(Ok(r)) => {
                lg(&l, format!("resp {} {:?}", r.status(), r.headers()));
                let mut body = r.into_body();
                loop {
                    match timeout(Duration::from_millis(300), body.data()).await {
                        Err(_) => { lg(&l, "data TIMEOUT".into()); break; }
                        Ok(None) => { lg(&l, "data None".into()); break; }
                        Ok(Some(Ok(d))) => { lg(&l, format!("data {} bytes", d.len())); }
                        Ok(Some(Err(e))) => { lg(&l, format!("data err {e:?}")); break; }
                    }
                }
                if trailers {
                    let t = timeout(Duration::from_millis(200), body.trailers()).await;
                    lg(&l, format!("trailers {t:?}"));
                }
            }
        }
    });
    let _ = dump(&mut x.s, 30).await;
    x.s.send_bytes(&mk).await;
    let fr = dump(&mut x.s, 100).await;
    let _ = t.await;
    report(name, &fr, x).await;
}
#[tokio::test]
async fn resp_validation() {
    let n = client::Builder::new;
    one_resp("no :status", n(), f(HEADERS, EH | ES, 1, &block(&[("x", "y")])), false, false).await;
    one_resp(":path in response", n(), f(HEADERS, EH | ES, 1, &block(&[(":path", "/"), (":status", "200")])), false, false).await;
    one_resp("304 cl + empty data", n(), [f(HEADERS, EH, 1, &resp("304", &[("content-length", "100")])), f(DATA, ES, 1, b"")].concat(), false, false).await;
    one_resp("304 cl + eos on headers (control)", n(), f(HEADERS, EH | ES, 1, &resp("304", &[("content-length", "100")])), false, false).await;
    one_resp("1xx cl then 200", n(), [f(HEADERS, EH, 1, &resp("103", &[("content-length", "0")])), f(HEADERS, EH, 1, &resp("200", &[])), f(DATA, ES, 1, b"0123456789")].concat(), false, false).await;
    one_resp("101", n(), f(HEADERS, EH, 1, &resp("101", &[])), false, false).await;
    one_resp("RST NO_ERROR after complete", n(), [f(HEADERS, EH, 1, &resp("200", &[])), f(DATA, ES, 1, b"12345678"), rst(1, 0)].concat(), true, true).await;
    one_resp("complete (control)", n(), [f(HEADERS, EH, 1, &resp("200", &[])), f(DATA, ES, 1, b"12345678")].concat(), true, true).await;
    let mut b = n();
    b.max_header_list_size(300);
    let big = "z".repeat(100);
    one_resp("oversize trailers", b, [f(HEADERS, EH, 1, &resp("200", &[])), f(HEADERS, EH | ES, 1, &block(&[("grpc-keep", "1"), ("x-big1", &big), ("x-big2", &big), ("x-big3", &big), ("grpc-status", "13")]))].concat(), true, false).await;
    one_resp("pseudo in trailers", n(), [f(HEADERS, EH, 1, &resp("200", &[])), f(HEADERS, EH | ES, 1, &block(&[(":path", "/"), ("x-ok", "1")]))].concat(), true, false).await;
    let mut b = n();
    b.max_header_list_size(300);
    one_resp("oversize response headers (control)", b, f(HEADERS, EH | ES, 1, &resp("200", &[("x-big1", &big), ("x-big2", &big), ("x-big3", &big)])), false, false).await;
}
#[tokio::test]
async fn eof_after_complete() {
    let mut x = cli(client::Builder::new()).await;
    let (resp, _st) = x.c.send_request(post(), false).unwrap();
    let _ = dump(&mut x.s, 30).await;
    x.s.send_bytes(&[f(HEADERS, EH, 1, &resp_b("200")), f(DATA, ES, 1, b"12345678")].concat()).await;
    let r = resp.await.unwrap();
    let mut body = r.into_body();
    sleep(Duration::from_millis(30)).await;
    drop(x.s);
    sleep(Duration::from_millis(30)).await;
    while let Some(d) = body.data().await { println!("eof_after_complete data={:?}", d.as_ref().map(|b| b.len())); if d.is_err() { break; } }
    println!("eof_after_complete end");
}
#[tokio::test]
async fn pushed_head() {
    let mut x = cli(client::Builder::new()).await;
    let (mut resp, _) = x.c.send_request(rq(), true).unwrap();
    let mut pushes = resp.push_promises();
    let l = x.l.clone();
    let t = tokio::spawn(async move {
        let p = timeout(Duration::from_millis(300), pushes.next()).await;
        if let Ok(Some(Ok(p))) = p {
            let (rq, rs) = p.into_parts();
            lg(&l, format!("promise {}", rq.method()));
            let r = timeout(Duration::from_millis(200), rs).await;
            lg(&l, format!("pushed resp {:?}", r.map(|x| x.map(|y| y.status()))));
        } else { lg(&l, format!("promise fail")); }
    });
    let _ = dump(&mut x.s, 30).await;
    let head = block(&[(":method", "HEAD"), (":scheme", "https"), (":path", "/"), (":authority", "a.test")]);
    let inp = [f(HEADERS, EH, 1, &resp_b("200")), f(PP, EH, 1, &pp_payload(2, &head)), f(HEADERS, EH | ES, 2, &probe::resp("200", &[("content-length", "100")]))].concat();
    x.s.send_bytes(&inp).await;
    let fr = dump(&mut x.s, 100).await;
    let _ = t.await;
    report("pushed HEAD w/ cl", &fr, x).await;
}
#[tokio::test]
async fn cl_control_head() {
    let mut x = cli(client::Builder::new()).await;
    let r = Request::builder().method("HEAD").uri("https://a.test/").body(()).unwrap();
    let (resp, _) = x.c.send_request(r, true).unwrap();
    let _ = dump(&mut x.s, 30).await;
    x.s.send_bytes(&f(HEADERS, EH | ES, 1, &probe::resp("200", &[("content-length", "100")]))).await;
    println!("control HEAD resp {:?}", resp.await.map(|r| r.status()));
}
#[tokio::test]
async fn trailers_hang() {
    let mut x = cli(client::Builder::new()).await;
    let (resp, _) = x.c.send_request(rq(), true).unwrap();
    let _ = dump(&mut x.s, 30).await;
    x.s.send_bytes(&[f(HEADERS, EH, 1, &resp_b("200")), f(DATA, 0, 1, b"unread"), f(HEADERS, EH | ES, 1, &block(&[("x-t", "1")]))].concat()).await;
    let r = resp.await.unwrap();
    let mut body = r.into_body();
    sleep(Duration::from_millis(30)).await;
    let t = timeout(Duration::from_millis(400), body.trailers()).await;
    println!("trailers w/ unread data: {t:?}");
    // manual: is waker registered? data was already there, nothing to wake.
    let d = body.data().await;
    println!("data: {:?}", d.map(|x| x.map(|b| b.len())));
    println!("after drain: {:?}", timeout(Duration::from_millis(100), body.trailers()).await);
}
#[tokio::test]
async fn goaway_scope_server_side() {
    // client mock sends request, then GOAWAY(0, NO_ERROR)
    let (io, mut c) = mock::new();
    let l = log();
    let l2 = l.clone();
    let j = tokio::spawn(async move {
        let mut conn = server::handshake(io).await.unwrap();
        let (_rq, mut sr) = conn.accept().await.unwrap().unwrap();
        lg(&l2, "accepted".into());
        let mut conn2 = conn;
        let r = tokio::spawn(async move { let r = conn2.accept().await; (conn2, r.map(|x| x.map(|_| ()))) });
        sleep(Duration::from_millis(80)).await;
        let rs = sr.send_response(Response::new(()), true);
        lg(&l2, format!("send_response {:?}", rs.map(|_| ())));
        let x = r.await.unwrap();
        lg(&l2, format!("accept {:?}", x.1));
    });
    c.assert_server_handshake().await;
    c.send_bytes(&f(HEADERS, EH | ES, 1, &req(&[]))).await;
    sleep(Duration::from_millis(30)).await;
    c.send_bytes(&goaway(0, 0, 0)).await;
    let fr = dump(&mut c, 200).await;
    show("client goaway(0) w/ request in flight", &fr, &l);
    j.abort();
}
#[tokio::test]
async fn goaway_scope_client_side() {
    let mut x = cli(client::Builder::new()).await;
    let (mut resp, _) = x.c.send_request(rq(), true).unwrap();
    let mut pushes = resp.push_promises();
    let l = x.l.clone();
    let t = tokio::spawn(async move {
        let _rr = resp;
        let p = pushes.next().await.unwrap().unwrap();
        let (_, rs) = p.into_parts();
        let r = rs.await.unwrap();
        let mut b = r.into_body();
        let d = timeout(Duration::from_millis(300), b.data()).await;
        lg(&l, format!("pushed body: {d:?}"));
    });
    let _ = dump(&mut x.s, 30).await;
    x.s.send_bytes(&[f(HEADERS, EH, 1, &resp_b("200")), pp(1, 2), f(HEADERS, EH, 2, &resp_b("200"))].concat()).await;
    sleep(Duration::from_millis(50)).await;
    x.s.send_bytes(&goaway(0, 1, 0)).await;
    x.s.send_bytes(&f(DATA, ES, 2, b"hello")).await;
    let fr = dump(&mut x.s, 100).await;
    let _ = t.await;
    report("server goaway(1) w/ pushed stream 2", &fr, x).await;
}
#[tokio::test]
async fn reset_before_flush_mock() {
    let mut x = cli(client::Builder::new()).await;
    let (_resp, mut st) = x.c.send_request(post(), false).unwrap();
    st.send_reset(Reason::CANCEL);
    let fr = dump(&mut x.s, 100).await;
    report("send_reset right after send_request (wire)", &fr, x).await;
}
#[tokio::test]
async fn reset_before_flush_real() {
    let (a, b) = tokio::io::duplex(1 << 20);
    let srv = tokio::spawn(async move {
        let mut c = server::handshake(b).await.unwrap();
        while let Some(r) = c.accept().await {
            match r { Ok((_, mut sr)) => { let _ = sr.send_response(Response::new(()), true); }, Err(e) => { println!("server accept err {e:?}"); break } }
        }
    });
    let (mut c, conn) = client::handshake(a).await.unwrap();
    let cj = tokio::spawn(async move { println!("client conn {:?}", conn.await); });
    let (r1, _s1) = c.send_request(post(), false).unwrap();
    sleep(Duration::from_millis(30)).await;
    let mut c = c.ready().await.unwrap();
    let (_r2, mut s2) = c.send_request(post(), false).unwrap();
    s2.send_reset(Reason::CANCEL);
    sleep(Duration::from_millis(50)).await;
    println!("sibling: {:?}", timeout(Duration::from_millis(100), r1).await.map(|r| r.map(|x| x.status())));
    let _ = (srv, cj);
}
#[tokio::test]
async fn reserve_pending_open() {
    let mut st = frame_settings();
    st.set_max_concurrent_streams(Some(1));
    let mut x = cli_set(client::Builder::new(), st).await;
    let (_r1, mut s1) = x.c.send_request(post(), false).unwrap();
    let a = dump(&mut x.s, 30).await;
    let mut c = x.c.clone().ready().await.unwrap();
    let (_r2, mut s2) = c.send_request(post(), false).unwrap();
    s2.reserve_capacity(65535);
    sleep(Duration::from_millis(30)).await;
    println!("s2 capacity={}", s2.capacity());
    s1.reserve_capacity(1000);
    sleep(Duration::from_millis(30)).await;
    println!("s1 capacity={}", s1.capacity());
    let _ = s1.send_data(Bytes::from(vec![b'x'; 1000]), false);
    let fr = dump(&mut x.s, 100).await;
    println!("reserve pending open: before={a:?} wire={fr:?}");
    // control: without reserve on s2
    let mut st = frame_settings();
    st.set_max_concurrent_streams(Some(1));
    let mut x = cli_set(client::Builder::new(), st).await;
    let (_r1, mut s1) = x.c.send_request(post(), false).unwrap();
    let _ = dump(&mut x.s, 30).await;
    let mut c = x.c.clone().ready().await.unwrap();
    let (_r2, _s2) = c.send_request(post(), false).unwrap();
    s1.reserve_capacity(1000);
    sleep(Duration::from_millis(30)).await;
    println!("control s1 capacity={}", s1.capacity());
}
#[tokio::test]
async fn waker_slot() {
    for with_a in [true, false] {
        let mut st = frame_settings();
        st.set_max_concurrent_streams(Some(1));
        let mut x = cli_set(client::Builder::new(), st).await;
        let (_r1, _s1) = x.c.send_request(post(), false).unwrap();
        let _ = dump(&mut x.s, 30).await;
        let mut c = x.c.clone().ready().await.unwrap();
        let (_r2, mut s2) = c.send_request(post(), false).unwrap();
        let t0 = Instant::now();
        let b = tokio::spawn(async move {
            let r = timeout(Duration::from_millis(800), c.ready()).await;
            println!("with_a={with_a} ready: is_ok={} after {:?}", r.is_ok(), t0.elapsed());
        });
        sleep(Duration::from_millis(20)).await;
        let a = if with_a { Some(tokio::spawn(async move {
            let r = timeout(Duration::from_millis(900), poll_fn(|cx| s2.poll_reset(cx))).await;
            println!("poll_reset {r:?}");
            drop(s2);
        })) } else { None };
        sleep(Duration::from_millis(20)).await;
        // finish stream 1 -> stream 3 opens
        x.s.send_bytes(&rst(1, 0)).await;
        let fr = dump(&mut x.s, 100).await;
        println!("wire: {fr:?}");
        let _ = b.await;
        drop(a);
    }
}
#[tokio::test]
async fn lib_reset_no_wake() {
    for lib in [true, false] {
        let mut x = cli(client::Builder::new()).await;
        let (resp, mut st) = x.c.send_request(post(), false).unwrap();
        let t = tokio::spawn(async move {
            let r = timeout(Duration::from_millis(500), poll_fn(|cx| st.poll_reset(cx))).await;
            println!("lib={lib} poll_reset: {r:?}");
        });
        let t2 = tokio::spawn(async move { let _ = resp.await.map(|r| r.into_body()); sleep(Duration::from_secs(1)).await; });
        let _ = dump(&mut x.s, 30).await;
        if lib {
            x.s.send_bytes(&[f(HEADERS, EH, 1, &probe::resp("200", &[("content-length", "1")])), f(DATA, 0, 1, b"xx")].concat()).await;
        } else {
            x.s.send_bytes(&rst(1, 8)).await;
        }
        let fr = dump(&mut x.s, 100).await;
        println!("wire: {fr:?}");
        let _ = t.await;
        t2.abort();
    }
}
#[tokio::test]
async fn reserve_trunc() {
    let mut x = cli(client::Builder::new()).await;
    let (_r, mut s) = x.c.send_request(post(), false).unwrap();
    s.reserve_capacity((1usize << 32) + 10);
    sleep(Duration::from_millis(20)).await;
    println!("cap after 2^32+10 = {}", s.capacity());
    let _ = &mut x;
}
#[tokio::test]
async fn send_side() {
    // send_trailers connection, te
    let mut x = cli(client::Builder::new()).await;
    let (_r, mut s) = x.c.send_request(post(), false).unwrap();
    let mut t = HeaderMap::new();
    t.insert("connection", "close".parse().unwrap());
    println!("send_trailers: {:?}", s.send_trailers(t));
    let fr = dump(&mut x.s, 60).await;
    println!("wire trailers: {fr:?}");
    let mut x = cli(client::Builder::new()).await;
    let r = Request::builder().uri("https://a.test/").header("te", "trailers").header("te", "gzip").body(()).unwrap();
    println!("te send: {:?}", x.c.send_request(r, true).map(|_| ()));
    let fr = dump(&mut x.s, 60).await;
    println!("wire te: {fr:?}");
    let mut x = cli(client::Builder::new()).await;
    let r = Request::builder().uri("https://a.test/").header("te", "gzip").body(()).unwrap();
    println!("te gzip control: {:?}", x.c.send_request(r, true).map(|_| ()));
    let _ = dump(&mut x.s, 30).await;
    for (nm, r) in [
        ("authority-form GET", Request::builder().uri("example.com").body(()).unwrap()),
        ("CONNECT /x", Request::builder().method("CONNECT").uri("/x").body(()).unwrap()),
        ("creds", Request::builder().uri("https://user:s3cret@x.com/").body(()).unwrap()),
        ("relative GET", Request::builder().uri("/x").body(()).unwrap()),
    ] {
        let mut x = cli(client::Builder::new()).await;
        println!("{nm}: {:?}", x.c.send_request(r, true).map(|_| ()));
        let fr = dump(&mut x.s, 60).await;
        println!("wire {nm}: {fr:?}");
    }
    let mut x = cli(client::Builder::new()).await;
    let mut r = Request::builder().uri("https://a.test/").body(()).unwrap();
    r.extensions_mut().insert(Protocol::from_static("websocket"));
    println!("protocol GET: {:?}", x.c.send_request(r, true).map(|_| ()));
    let fr = dump(&mut x.s, 60).await;
    println!("wire protocol GET: {fr:?}");
}
#[tokio::test]
async fn too_many_push_resets() {
    let mut x = cli(client::Builder::new()).await;
    let (mut resp, _) = x.c.send_request(rq(), true).unwrap();
    let mut pushes = resp.push_promises();
    let _ = dump(&mut x.s, 30).await;
    x.s.send_bytes(&f(HEADERS, EH, 1, &resp_b("200"))).await;
    for i in 0..30u32 {
        let id = 2 * i + 2;
        x.s.send_bytes(&[pp(1, id), rst(id, 8)].concat()).await;
        let a = timeout(Duration::from_millis(100), pushes.next()).await;
        let d = dump(&mut x.s, 10).await;
        let got = match &a { Ok(Some(Ok(_))) => "ok".to_string(), o => format!("{:?}", o.as_ref().map(|x| x.as_ref().map(|y| y.as_ref().map(|_| ())))) };
        println!("iter {i}: push={got} wire={d:?}");
        if d.iter().any(|s| s.contains("GoAway")) { break; }
    }
    let _ = resp;
}
#[tokio::test]
async fn push_window_leak_parent_drop() {
    let mut x = cli(client::Builder::new()).await;
    let (resp, _) = x.c.send_request(rq(), true).unwrap();
    let _ = dump(&mut x.s, 30).await;
    let mut inp = [f(HEADERS, EH, 1, &resp_b("200")), pp(1, 2), f(HEADERS, EH, 2, &resp_b("200"))].concat();
    for _ in 0..3 { inp.extend(f(DATA, 0, 2, &vec![b'x'; 16384])); }
    x.s.send_bytes(&inp).await;
    sleep(Duration::from_millis(50)).await;
    drop(resp);
    let fr = dump(&mut x.s, 300).await;
    report("push data, parent dropped", &fr, x).await;
}
#[tokio::test]
async fn pp_on_reset() {
    // drop response (implicit), then PP, H(2)
    let mut x = cli(client::Builder::new()).await;
    let (resp, _) = x.c.send_request(rq(), true).unwrap();
    let _ = dump(&mut x.s, 30).await;
    x.s.send_bytes(&f(HEADERS, EH, 1, &resp_b("200"))).await;
    sleep(Duration::from_millis(30)).await;
    drop(resp);
    let a = dump(&mut x.s, 50).await;
    x.s.send_bytes(&[pp(1, 2), f(HEADERS, EH, 2, &resp_b("200"))].concat()).await;
    let fr = dump(&mut x.s, 100).await;
    report("PP after drop", &[a, fr].concat(), x).await;
    // drop before flush: implicit reset pending
    let mut x = cli(client::Builder::new()).await;
    let (resp, _) = x.c.send_request(post(), false).unwrap();
    let (_r2, _) = x.c.send_request(rq(), true).unwrap();
    let _ = dump(&mut x.s, 30).await;
    x.s.send_bytes(&f(HEADERS, EH, 1, &resp_b("200"))).await;
    x.s.send_bytes(&[pp(3, 2)].concat()).await;
    drop(resp);
    let fr = dump(&mut x.s, 100).await;
    report("control drop no pp on it", &fr, x).await;
}
#[tokio::test]
async fn pp_on_user_reset() {
    let mut x = cli(client::Builder::new()).await;
    let (_resp, mut st) = x.c.send_request(post(), false).unwrap();
    let _ = dump(&mut x.s, 30).await;
    x.s.send_bytes(&f(HEADERS, EH, 1, &resp_b("200"))).await;
    sleep(Duration::from_millis(30)).await;
    st.send_reset(Reason::CANCEL);
    let a = dump(&mut x.s, 50).await;
    x.s.send_bytes(&[pp(1, 2), f(HEADERS, EH, 2, &resp_b("200"))].concat()).await;
    let fr = dump(&mut x.s, 100).await;
    report("PP after user send_reset", &[a, fr].concat(), x).await;
}
#[tokio::test]
async fn pp_empty_first_fragment() {
    let mut x = cli(client::Builder::new()).await;
    let (_resp, _) = x.c.send_request(rq(), true).unwrap();
    let _ = dump(&mut x.s, 30).await;
    x.s.send_bytes(&[f(HEADERS, EH, 1, &resp_b("200")), f(PP, 0, 1, &2u32.to_be_bytes()), f(CONT, EH, 1, &req(&[]))].concat()).await;
    let fr = dump(&mut x.s, 100).await;
    report("PP empty first frag", &fr, x).await;
    let mut x = cli(client::Builder::new()).await;
    let (_resp, _) = x.c.send_request(rq(), true).unwrap();
    let _ = dump(&mut x.s, 30).await;
    let b = req(&[]);
    x.s.send_bytes(&[f(HEADERS, EH, 1, &resp_b("200")), f(PP, 0, 1, &pp_payload(2, &b[..3])), f(CONT, EH, 1, &b[3..])].concat()).await;
    let fr = dump(&mut x.s, 100).await;
    report("PP nonempty first frag control", &fr, x).await;
}
#[tokio::test]
async fn pushes_unbounded() {
    let mut b = client::Builder::new();
    b.max_concurrent_streams(10);
    let (io, mut s) = mock::new();
    let (tx, rx) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        let (c, conn) = b.handshake::<_, Bytes>(io).await.unwrap();
        let _ = tx.send(c);
        let _ = conn.await;
    });
    s.assert_client_handshake().await;
    let mut c = rx.await.unwrap();
    let (_resp, _) = c.send_request(rq(), true).unwrap();
    sleep(Duration::from_millis(20)).await;
    s.send_bytes(&f(HEADERS, EH, 1, &resp_b("200"))).await;
    for i in 0..5000u32 { s.send_bytes(&pp(1, 2 * i + 2)).await; }
    sleep(Duration::from_millis(200)).await;
    println!("wired after 5000 PP: {}", c.num_wired_streams());
}
