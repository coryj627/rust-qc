use probe::*;
fn rq() -> Request<()> { req_get("https://a.test/") }
#[tokio::test]
async fn same_task() {
    let (io, mut s) = mock::new();
    let mock = async move {
        s.assert_client_handshake().await;
        let _ = dump(&mut s, 30).await;
        s.send_bytes(&f(HEADERS, EH, 1, &resp("200", &[]))).await;
        sleep(Duration::from_millis(50)).await;
        s.send_bytes(&[pp(1, 2), f(HEADERS, EH, 2, &resp("100", &[])), f(HEADERS, EH, 2, &resp("200", &[]))].concat()).await;
        sleep(Duration::from_secs(5)).await;
    };
    let cl = async move {
        let (mut c, mut conn) = client::handshake(io).await.unwrap();
        let (r, _) = c.send_request(rq(), true).unwrap();
        let resp = tokio::select! { x = r => x.unwrap(), _ = &mut conn => panic!("done") };
        let body = resp.into_body();
        println!("have body; driving conn in same task");
        let _ = (&mut conn).await;
        drop(body);
    };
    let t = tokio::spawn(cl);
    mock.await;
    let _ = t.await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn separate_tasks() {
    // connection in its own task; user task holds a RecvStream and has a timer re-poll
    let mut x = cli(client::Builder::new()).await;
    let (r, _) = x.c.send_request(rq(), true).unwrap();
    let u = tokio::spawn(async move {
        let resp = r.await.unwrap();
        let mut body = resp.into_body();
        loop {
            let d = timeout(Duration::from_millis(50), body.data()).await;
            println!("user poll: {:?}", d.is_ok());
        }
    });
    let _ = dump(&mut x.s, 30).await;
    x.s.send_bytes(&f(HEADERS, EH, 1, &resp("200", &[]))).await;
    sleep(Duration::from_millis(120)).await;
    x.s.send_bytes(&[pp(1, 2), f(HEADERS, EH, 2, &resp("100", &[])), f(HEADERS, EH, 2, &resp("200", &[]))].concat()).await;
    sleep(Duration::from_millis(500)).await;
    println!("SURVIVED conn_finished={} user={:?}", x.j.is_finished(), u.is_finished());
    println!("user join: {:?}", u.await.map(|_| ()).map_err(|e| e.is_panic()));
}
#[tokio::test]
async fn server_hdr_panic_no_poison() {
    // many names panic: does the mutex get poisoned? handler polls body after
    let (io, mut c) = mock::new();
    let j = tokio::spawn(async move {
        let mut conn = server::handshake(io).await.unwrap();
        let (rq, _sr) = conn.accept().await.unwrap().unwrap();
        let u = tokio::spawn(async move {
            let mut b = rq.into_body();
            loop { let d = timeout(Duration::from_millis(50), b.data()).await; println!("user poll {:?}", d.map(|x| x.map(|y| y.map(|_| ()))).ok()); if let Ok(_)=timeout(Duration::from_millis(0), futures::future::pending::<()>()).await {} }
        });
        let _ = conn.accept().await;
        drop(u);
    });
    c.assert_server_handshake().await;
    c.send_bytes(&f(HEADERS, EH, 1, &req(&[]))).await;
    sleep(Duration::from_millis(50)).await;
    let mut hb = req(&[]);
    for i in 0..30000 { hb.extend(lit(&format!("x-{i:07}"), "")); }
    let chunks: Vec<_> = hb.chunks(16384).collect();
    let mut out = vec![];
    for (i, ch) in chunks.iter().enumerate() {
        out.extend(f(if i == 0 { HEADERS } else { CONT }, if i == chunks.len() - 1 { EH | ES } else { 0 } | if i==0 {ES} else {0}, 3, ch));
    }
    c.send_bytes(&out).await;
    sleep(Duration::from_millis(300)).await;
    println!("conn panic: {:?}", j.await.map_err(|e| e.is_panic()));
}
