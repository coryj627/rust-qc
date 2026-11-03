use probe::*;
fn rq() -> Request<()> { req_get("https://a.test/") }
fn post() -> Request<()> { Request::builder().method("POST").uri("https://a.test/").body(()).unwrap() }
#[tokio::test]
async fn lib_reset_timing() {
    for lib in [true, false] {
        let mut x = cli(client::Builder::new()).await;
        let (resp, mut st) = x.c.send_request(post(), false).unwrap();
        let t0 = Instant::now();
        let t = tokio::spawn(async move {
            let r = timeout(Duration::from_millis(600), poll_fn(|cx| st.poll_reset(cx))).await;
            println!("lib={lib} poll_reset: {r:?} after {:?}", t0.elapsed());
        });
        let _t2 = tokio::spawn(async move { let r = resp.await; let b = r.map(|r| r.into_body()); sleep(Duration::from_secs(1)).await; drop(b); });
        let _ = dump(&mut x.s, 30).await;
        if lib {
            x.s.send_bytes(&[f(HEADERS, EH, 1, &probe::resp("200", &[("content-length", "1")])), f(DATA, 0, 1, b"xx")].concat()).await;
        } else {
            x.s.send_bytes(&rst(1, 8)).await;
        }
        let _ = dump(&mut x.s, 100).await;
        let _ = t.await;
    }
    // poll_capacity shares the waker
    let mut x = cli(client::Builder::new()).await;
    let (resp, mut st) = x.c.send_request(post(), false).unwrap();
    st.reserve_capacity(100000);
    let t0 = Instant::now();
    let t = tokio::spawn(async move {
        let _ = st.capacity();
        let a = poll_fn(|cx| st.poll_capacity(cx)).await;
        println!("first cap {a:?}");
        let r = timeout(Duration::from_millis(600), poll_fn(|cx| st.poll_capacity(cx))).await;
        println!("lib poll_capacity: {r:?} after {:?}", t0.elapsed());
    });
    let _t2 = tokio::spawn(async move { let r = resp.await; let b = r.map(|r| r.into_body()); sleep(Duration::from_secs(1)).await; drop(b); });
    let _ = dump(&mut x.s, 30).await;
    x.s.send_bytes(&[f(HEADERS, EH, 1, &probe::resp("200", &[("content-length", "1")])), f(DATA, 0, 1, b"xx")].concat()).await;
    let _ = dump(&mut x.s, 100).await;
    let _ = t.await;
}
async fn wire_pseudo(nm: &str, r: Request<()>) {
    let mut x = cli(client::Builder::new()).await;
    let res = x.c.send_request(r, true).map(|_| ());
    let fr = timeout(Duration::from_millis(100), x.s.next()).await;
    if let Ok(Some(Ok(h2::frame::Frame::Headers(h)))) = fr {
        let (p, _) = h.into_parts();
        println!("{nm}: {res:?} pseudo={p:?}");
    } else { println!("{nm}: {res:?} other={fr:?}"); }
}
#[tokio::test]
async fn pseudo_wire() {
    wire_pseudo("authority-form GET", Request::builder().uri("example.com").body(()).unwrap()).await;
    wire_pseudo("CONNECT /x", Request::builder().method("CONNECT").uri("/x").body(()).unwrap()).await;
    wire_pseudo("creds", Request::builder().uri("https://user:s3cret@x.com/").body(()).unwrap()).await;
    wire_pseudo("relative GET", Request::builder().uri("/x").body(()).unwrap()).await;
}
#[tokio::test]
async fn forgot_data_default() {
    let (io, mut c) = mock::new();
    let l = log();
    let j = spawn_srv(server::Builder::new(), io, l.clone());
    c.assert_server_handshake().await;
    c.send_bytes(&f(HEADERS, EH, 1, &req(&[("connection", "close")]))).await;
    let a = dump(&mut c, 50).await;
    sleep(Duration::from_millis(1100)).await;
    c.send_bytes(&f(6, 0, 0, &[0; 8])).await;
    let _ = dump(&mut c, 100).await;
    c.send_bytes(&f(DATA, 0, 1, b"x")).await;
    let fr = dump(&mut c, 200).await;
    show("forgotten DATA default timing", &[a, fr].concat(), &l);
    j.abort();
}
#[tokio::test]
async fn pending_capacity_evict() {
    let mut x = cli(client::Builder::new()).await;
    let (_r1, mut s1) = x.c.send_request(post(), false).unwrap();
    s1.reserve_capacity(65535);
    let mut c = x.c.clone().ready().await.unwrap();
    let (r3, mut s3) = c.send_request(post(), false).unwrap();
    let mut c = c.ready().await.unwrap();
    let (r5, mut s5) = c.send_request(post(), false).unwrap();
    s3.reserve_capacity(10);
    s5.reserve_capacity(10);
    let _ = dump(&mut x.s, 30).await;
    println!("s1={} s3={} s5={} wired={}", s1.capacity(), s3.capacity(), s5.capacity(), x.c.num_wired_streams());
    x.s.send_bytes(&[rst(3, 8), rst(5, 8)].concat()).await;
    sleep(Duration::from_millis(30)).await;
    drop(r3); drop(s3); drop(r5); drop(s5);
    sleep(Duration::from_millis(30)).await;
    println!("after rst+drop: wired={} active={}", x.c.num_wired_streams(), x.c.num_active_streams());
    x.s.send_bytes(&wu(0, 1000)).await;
    sleep(Duration::from_millis(30)).await;
    println!("after conn WU: wired={} active={}", x.c.num_wired_streams(), x.c.num_active_streams());
    // nudge
    x.s.send_bytes(&f(6, 0, 0, &[0; 8])).await;
    sleep(Duration::from_millis(30)).await;
    println!("after ping: wired={} active={}", x.c.num_wired_streams(), x.c.num_active_streams());
}
async fn reset_counter(with_data: bool) {
    let mut st = frame_settings();
    st.set_initial_window_size(Some(0));
    let mut b = client::Builder::new();
    b.max_concurrent_reset_streams(1);
    b.reset_stream_duration(Duration::from_millis(100));
    let mut x = cli_set(b, st).await;
    let (r1, mut s1) = x.c.send_request(post(), false).unwrap();
    if with_data { s1.send_data(Bytes::from_static(b"0123456789"), false).unwrap(); }
    let a = dump(&mut x.s, 30).await;
    drop(r1); drop(s1);
    let a2 = dump(&mut x.s, 30).await;
    sleep(Duration::from_millis(150)).await;
    x.s.send_bytes(&f(6, 0, 0, &[0; 8])).await;
    let a3 = dump(&mut x.s, 30).await;
    let mut c = x.c.clone().ready().await.unwrap();
    let (r3, mut s3) = c.send_request(post(), false).unwrap();
    let a4 = dump(&mut x.s, 30).await;
    s3.send_reset(Reason::CANCEL);
    let a5 = dump(&mut x.s, 20).await;
    x.s.send_bytes(&f(DATA, 0, 3, b"late")).await;
    let a6 = dump(&mut x.s, 30).await;
    println!("reset_counter with_data={with_data}\n a={a:?}\n drop={a2:?}\n after-expiry={a3:?}\n open3={a4:?}\n reset3={a5:?}\n late={a6:?}");
    drop(r3);
}
#[tokio::test]
async fn reset_counter_leak() { reset_counter(true).await; reset_counter(false).await; }
#[tokio::test]
async fn implicit_reset_capacity() {
    let mut st = frame_settings();
    st.set_max_concurrent_streams(Some(1));
    let mut x = cli_set(client::Builder::new(), st).await;
    let (_r1, _s1) = x.c.send_request(post(), false).unwrap();
    let _ = dump(&mut x.s, 30).await;
    let mut c = x.c.clone().ready().await.unwrap();
    let (r2, mut s2) = c.send_request(post(), false).unwrap();
    s2.reserve_capacity(60000);
    sleep(Duration::from_millis(20)).await;
    println!("s2 cap {}", s2.capacity());
    drop(r2); drop(s2);
    drop(c);
    sleep(Duration::from_millis(20)).await;
    x.s.send_bytes(&rst(1, 8)).await;
    let a = dump(&mut x.s, 50).await;
    println!("after stream1 ends: {a:?}");
    let mut c = x.c.clone().ready().await.unwrap();
    let (_r3, mut s3) = c.send_request(post(), false).unwrap();
    s3.send_data(Bytes::from(vec![b'z'; 20000]), false).unwrap();
    let b = dump(&mut x.s, 100).await;
    let n = b.iter().filter(|s| s.starts_with("Data")).count();
    println!("s3 wire ({n} data frames): {b:?}");
    println!("s3 cap {}", s3.capacity());
}
#[tokio::test]
async fn late_frames_cap() {
    let mut b = client::Builder::new();
    b.max_concurrent_reset_streams(0);
    let mut x = cli(b).await;
    let (r, mut s) = x.c.send_request(post(), false).unwrap();
    let _ = dump(&mut x.s, 30).await;
    s.send_reset(Reason::CANCEL);
    drop(r); drop(s);
    let _ = dump(&mut x.s, 30).await;
    let mut n = 0;
    for i in 0..1100 {
        x.s.send_bytes(&f(DATA, 0, 1, b"late")).await;
        let d = dump(&mut x.s, 2).await;
        if d.iter().any(|s| s.contains("GoAway")) { println!("late frames: GOAWAY after {} frames: {d:?}", i + 1); n = i + 1; break; }
    }
    println!("late frames n={n}");
}
#[tokio::test]
async fn cancel_delayed() {
    let mut st = frame_settings();
    st.set_max_concurrent_streams(Some(1));
    let mut x = cli_set(client::Builder::new(), st).await;
    let (_r1, _s1) = x.c.send_request(post(), false).unwrap();
    let _ = dump(&mut x.s, 30).await;
    let mut c = x.c.clone().ready().await.unwrap();
    let (r2, s2) = c.send_request(post(), false).unwrap();
    x.s.send_bytes(&rst(1, 8)).await;
    let a = dump(&mut x.s, 30).await;
    println!("after s1 end: {a:?}");
    drop(r2); drop(s2);
    let b = dump(&mut x.s, 100).await;
    println!("after drop w/ SendRequest held: {b:?}");
    let c2 = c.ready().await;
    let e = dump(&mut x.s, 50).await;
    println!("after poll_ready (ok={}): {e:?}", c2.is_ok());
}
#[tokio::test]
async fn dup_frames_goaway_other() {
    // WINDOW_UPDATE of 0 on conn; GOAWAY with nonzero handled by client side as well
    let mut x = cli(client::Builder::new()).await;
    let (r, _) = x.c.send_request(rq(), true).unwrap();
    let _ = dump(&mut x.s, 30).await;
    x.s.send_bytes(&goaway(5, 0, 0)).await;
    let fr = dump(&mut x.s, 50).await;
    println!("client: GOAWAY on stream 5: {fr:?}; resp={:?}", r.await.map(|x| x.status()));
}

#[tokio::test(flavor = "current_thread")]
async fn pp_cross_scheduled_reset() {
    let mut x = cli(client::Builder::new()).await;
    let (resp, _) = x.c.send_request(rq(), true).unwrap();
    let (_r3, _) = x.c.clone().send_request(rq(), true).unwrap();
    let _ = dump(&mut x.s, 30).await;
    x.s.send_bytes(&f(HEADERS, EH, 1, &probe::resp("200", &[]))).await;
    sleep(Duration::from_millis(20)).await;
    x.s.send_bytes(&pp(1, 2)).await;
    drop(resp);
    let fr = dump(&mut x.s, 100).await;
    report("PP crossing scheduled implicit reset", &fr, x).await;
}
#[tokio::test]
async fn pp_on_user_reset_rel() {
    let mut x = cli(client::Builder::new()).await;
    let (_resp, mut st) = x.c.send_request(post(), false).unwrap();
    let _ = dump(&mut x.s, 30).await;
    x.s.send_bytes(&f(HEADERS, EH, 1, &probe::resp("200", &[]))).await;
    sleep(Duration::from_millis(30)).await;
    st.send_reset(Reason::CANCEL);
    let a = dump(&mut x.s, 50).await;
    x.s.send_bytes(&pp(1, 2)).await;
    let m = dump(&mut x.s, 50).await;
    x.s.send_bytes(&f(HEADERS, EH, 2, &probe::resp("200", &[]))).await;
    let fr = dump(&mut x.s, 100).await;
    report("PP after user send_reset", &[a, m, fr].concat(), x).await;
}
#[tokio::test]
async fn pending_open_reset_wire() {
    let mut st = frame_settings();
    st.set_max_concurrent_streams(Some(1));
    let mut x = cli_set(client::Builder::new(), st).await;
    let (_r1, _s1) = x.c.send_request(post(), false).unwrap();
    let _ = dump(&mut x.s, 30).await;
    let mut c = x.c.clone().ready().await.unwrap();
    let (_r2, mut s2) = c.send_request(post(), false).unwrap();
    sleep(Duration::from_millis(10)).await;
    s2.send_reset(Reason::CANCEL);
    let fr = dump(&mut x.s, 60).await;
    println!("pending_open send_reset wire: {fr:?}");
}
#[tokio::test]
async fn capacity_leak_bytes() {
    let mut st = frame_settings();
    st.set_max_concurrent_streams(Some(1));
    let mut x = cli_set(client::Builder::new(), st).await;
    let (_r1, _s1) = x.c.send_request(post(), false).unwrap();
    let _ = dump(&mut x.s, 30).await;
    let mut c = x.c.clone().ready().await.unwrap();
    let (r2, mut s2) = c.send_request(post(), false).unwrap();
    s2.send_data(Bytes::from(vec![b'q'; 60000]), false).unwrap();
    sleep(Duration::from_millis(20)).await;
    println!("queued send_data s2 cap {}", s2.capacity());
    drop(r2); drop(s2); drop(c);
    sleep(Duration::from_millis(20)).await;
    x.s.send_bytes(&rst(1, 8)).await;
    let a = dump(&mut x.s, 50).await;
    println!("after s1 end: {a:?}");
    let mut c = x.c.clone().ready().await.unwrap();
    let (_r3, mut s3) = c.send_request(post(), false).unwrap();
    s3.send_data(Bytes::from(vec![b'z'; 20000]), false).unwrap();
    let mut got = 0;
    loop {
        match timeout(Duration::from_millis(150), x.s.next()).await {
            Ok(Some(Ok(h2::frame::Frame::Data(d)))) => got += d.payload().len(),
            Ok(Some(Ok(_))) => {}
            _ => break,
        }
    }
    println!("capacity_leak: s3 delivered {got} of 20000");
}
#[tokio::test]
async fn client_many_names() {
    let mut x = cli(client::Builder::new()).await;
    let (_r, _) = x.c.send_request(rq(), true).unwrap();
    let _ = dump(&mut x.s, 30).await;
    let mut hb = probe::resp("200", &[]);
    for i in 0..30000 { hb.extend(lit(&format!("x-{i:07}"), "")); }
    let chunks: Vec<_> = hb.chunks(16384).collect();
    let mut out = vec![];
    for (i, ch) in chunks.iter().enumerate() {
        let mut fl = if i == chunks.len() - 1 { EH } else { 0 };
        if i == 0 { fl |= ES; }
        out.extend(f(if i == 0 { HEADERS } else { CONT }, fl, 1, ch));
    }
    x.s.send_bytes(&out).await;
    let fr = dump(&mut x.s, 100).await;
    report("client many names", &fr, x).await;
}
#[tokio::test]
async fn pending_open_reset_then_open() {
    let mut st = frame_settings();
    st.set_max_concurrent_streams(Some(1));
    let mut x = cli_set(client::Builder::new(), st).await;
    let (_r1, _s1) = x.c.send_request(post(), false).unwrap();
    let _ = dump(&mut x.s, 30).await;
    let mut c = x.c.clone().ready().await.unwrap();
    let (_r2, mut s2) = c.send_request(post(), false).unwrap();
    sleep(Duration::from_millis(10)).await;
    s2.send_reset(Reason::CANCEL);
    let _ = dump(&mut x.s, 30).await;
    x.s.send_bytes(&rst(1, 8)).await;
    let fr = dump(&mut x.s, 60).await;
    println!("pending_open send_reset after slot frees, wire: {fr:?}");
}
#[tokio::test]
async fn capacity_leak_reserve_only() {
    let mut st = frame_settings();
    st.set_max_concurrent_streams(Some(1));
    let mut x = cli_set(client::Builder::new(), st).await;
    let (_r1, _s1) = x.c.send_request(post(), false).unwrap();
    let _ = dump(&mut x.s, 30).await;
    let mut c = x.c.clone().ready().await.unwrap();
    let (r2, mut s2) = c.send_request(post(), false).unwrap();
    s2.reserve_capacity(60000);
    sleep(Duration::from_millis(20)).await;
    drop(r2); drop(s2); drop(c);
    sleep(Duration::from_millis(20)).await;
    x.s.send_bytes(&rst(1, 8)).await;
    let a = dump(&mut x.s, 50).await;
    println!("after s1 end: {a:?}");
    let mut c = x.c.clone().ready().await.unwrap();
    let (_r3, mut s3) = c.send_request(post(), false).unwrap();
    s3.send_data(Bytes::from(vec![b'z'; 20000]), false).unwrap();
    let mut got = 0;
    loop {
        match timeout(Duration::from_millis(150), x.s.next()).await {
            Ok(Some(Ok(h2::frame::Frame::Data(d)))) => got += d.payload().len(),
            Ok(Some(Ok(_))) => {}
            _ => break,
        }
    }
    println!("capacity_leak_reserve_only: s3 delivered {got} of 20000 (no DATA was ever sent before it)");
    // does a later conn WU restore? 
    x.s.send_bytes(&wu(0, 1000)).await;
    let d = dump(&mut x.s, 60).await;
    println!("after WU 1000: {d:?}");
}
