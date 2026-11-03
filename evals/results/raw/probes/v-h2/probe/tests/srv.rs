use probe::*;

async fn srv_case(name: &str, b: server::Builder, input: Vec<u8>, ms: u64) -> (Vec<String>, Log) {
    let (io, mut c) = mock::new();
    let l = log();
    let j = spawn_srv(b, io, l.clone());
    c.assert_server_handshake().await;
    c.send_bytes(&input).await;
    let fr = dump(&mut c, ms).await;
    if j.is_finished() {
        match j.await {
            Err(e) if e.is_panic() => lg(&l, "SERVER TASK PANICKED".into()),
            _ => {}
        }
    }
    show(name, &fr, &l);
    (fr, l)
}
fn names(n: usize) -> Vec<u8> {
    let mut hb = req(&[]);
    for i in 0..n {
        hb.extend(lit(&format!("x-{i:07}"), ""));
    }
    let mut out = vec![];
    let chunks: Vec<_> = hb.chunks(16384).collect();
    let k = chunks.len();
    for (i, ch) in chunks.iter().enumerate() {
        let fl = if i == k - 1 { EH | ES } else { 0 };
        let fl2 = if i == 0 { fl } else { fl };
        out.extend(f(if i == 0 { HEADERS } else { CONT }, fl2 | if i == 0 { ES } else { 0 }, 1, ch));
    }
    out
}
#[tokio::test]
async fn names_20000() {
    srv_case("names 20000", server::Builder::new(), names(20000), 300).await;
}
#[tokio::test]
async fn names_24000() {
    let (_, l) = srv_case("names 24000", server::Builder::new(), names(24000), 300).await;
    let _ = l;
}
#[tokio::test]
async fn names_24600() {
    srv_case("names 24600", server::Builder::new(), names(24600), 300).await;
}
#[tokio::test]
async fn names_40000_limit_1m() {
    let mut b = server::Builder::new();
    b.max_header_list_size(1 << 20);
    srv_case("names 40000 limit 1M", b, names(40000), 300).await;
}
#[tokio::test]
async fn padding() {
    let mut inp = f(HEADERS, EH, 1, &req(&[]));
    for _ in 0..300 {
        let mut p = vec![254u8, b'x'];
        p.extend(vec![0u8; 254]);
        inp.extend(f(DATA, PADDED, 1, &p));
    }
    inp.extend(f(DATA, ES, 1, b""));
    srv_case("padding 76800 on 65535 window", server::Builder::new(), inp, 300).await;
}
#[tokio::test]
async fn padding_pure() {
    let mut inp = f(HEADERS, EH, 1, &req(&[]));
    for _ in 0..300 {
        let mut p = vec![255u8];
        p.extend(vec![0u8; 255]);
        inp.extend(f(DATA, PADDED, 1, &p));
    }
    for _ in 0..2 {
        let mut p = vec![200u8];
        p.extend(vec![b'y'; 16000]);
        p.extend(vec![0u8; 200]);
        inp.extend(f(DATA, PADDED, 1, &p));
    }
    inp.extend(f(DATA, ES, 1, b""));
    srv_case("padding pure + 2 big", server::Builder::new(), inp, 300).await;
}
async fn drop_body_case(all: bool) {
    let (io, mut c) = mock::new();
    let l = log();
    let l2 = l.clone();
    let j = tokio::spawn(async move {
        let mut conn = server::handshake(io).await.unwrap();
        let (req, mut sr) = conn.accept().await.unwrap().unwrap();
        let drive = async {
            sleep(Duration::from_millis(100)).await;
            drop(req);
            lg(&l2, "dropped body".into());
            let _st = sr.send_response(Response::new(()), false).unwrap();
            if all {
                drop(_st);
                drop(sr);
                futures::future::pending::<()>().await;
            }
            sleep(Duration::from_secs(10)).await;
        };
        let _ = futures::future::select(Box::pin(drive), Box::pin(async { while let Some(_) = conn.accept().await {} })).await;
    });
    c.assert_server_handshake().await;
    let mut inp = f(HEADERS, EH, 1, &req(&[]));
    for _ in 0..3 {
        inp.extend(f(DATA, 0, 1, &vec![b'x'; 16384]));
    }
    c.send_bytes(&inp).await;
    let fr = dump(&mut c, 400).await;
    j.abort();
    show(&format!("drop body all={all}"), &fr, &l);
}
#[tokio::test]
async fn drop_body_keep_send() {
    drop_body_case(false).await;
}
#[tokio::test]
async fn drop_body_all() {
    drop_body_case(true).await;
}
#[tokio::test]
async fn goaway_data() {
    let (io, mut c) = mock::new();
    let l = log();
    let l2 = l.clone();
    let j = tokio::spawn(async move {
        let mut conn = server::handshake(io).await.unwrap();
        let (req, _sr) = conn.accept().await.unwrap().unwrap();
        conn.graceful_shutdown();
        let mut body = req.into_body();
        let d = async {
            while let Some(Ok(d)) = body.data().await {
                let _ = body.flow_control().release_capacity(d.len());
                lg(&l2, format!("data {}", d.len()));
            }
            futures::future::pending::<()>().await;
        };
        let _ = futures::future::select(Box::pin(d), Box::pin(async { while let Some(_) = conn.accept().await {} })).await;
    });
    c.assert_server_handshake().await;
    c.send_bytes(&f(HEADERS, EH, 1, &req(&[]))).await;
    let a = dump(&mut c, 100).await;
    println!("before ping {a:#?}");
    // ping ack shutdown
    c.send_frame(frames::ping(h2::frame::Ping::SHUTDOWN).pong()).await;
    let b = dump(&mut c, 100).await;
    println!("after ping {b:#?}");
    let mut inp = f(HEADERS, EH, 3, &req(&[]));
    for _ in 0..3 {
        inp.extend(f(DATA, 0, 3, &vec![b'x'; 16384]));
    }
    c.send_bytes(&inp).await;
    let fr = dump(&mut c, 300).await;
    show("goaway: data on ignored stream 3", &fr, &l);
    // control: data on stream 1
    c.send_bytes(&f(DATA, 0, 1, &vec![b'x'; 16384])).await;
    c.send_bytes(&f(DATA, 0, 1, &vec![b'x'; 16384])).await;
    let fr = dump(&mut c, 300).await;
    show("goaway: control data on stream 1", &fr, &l);
    j.abort();
}
#[tokio::test]
async fn goaway_wu_ignored() {
    let (io, mut c) = mock::new();
    let l = log();
    let j = tokio::spawn(async move {
        let mut conn = server::handshake(io).await.unwrap();
        let (_req, _sr) = conn.accept().await.unwrap().unwrap();
        conn.graceful_shutdown();
        let r = conn.accept().await;
        println!("accept2 = {:?}", r.map(|x| x.map(|_| ())));
    });
    c.assert_server_handshake().await;
    c.send_bytes(&f(HEADERS, EH, 1, &req(&[]))).await;
    let _ = dump(&mut c, 100).await;
    c.send_frame(frames::ping(h2::frame::Ping::SHUTDOWN).pong()).await;
    let b = dump(&mut c, 100).await;
    println!("after ping {b:#?}");
    let mut inp = f(HEADERS, EH, 3, &req(&[]));
    inp.extend(wu(3, 100));
    c.send_bytes(&inp).await;
    let fr = dump(&mut c, 300).await;
    show("goaway: HEADERS(3) ignored + WU(3)", &fr, &l);
    j.abort();
}
#[tokio::test]
async fn malformed_then_continuation() {
    let mut inp = f(HEADERS, 0, 1, &req(&[("connection", "close")]));
    inp.extend(f(CONT, EH, 1, &lit("x", "y")));
    inp.extend(f(HEADERS, EH | ES, 3, &req(&[])));
    srv_case("malformed + continuation", server::Builder::new(), inp, 200).await;
}
#[tokio::test]
async fn malformed_oneframe_control() {
    let mut inp = f(HEADERS, EH, 1, &req(&[("connection", "close")]));
    inp.extend(f(HEADERS, EH | ES, 3, &req(&[])));
    srv_case("malformed one frame control", server::Builder::new(), inp, 200).await;
}
#[tokio::test]
async fn malformed_split_lost() {
    for (nm, bad) in [("conn", lit("connection", "close")), ("dup path", lit(":path", "/x")), ("te", lit("te", "gzip"))] {
        let mut hb = req(&[]);
        hb.extend(bad);
        let tail = lit("x-tail", "value-long-value");
        hb.extend(&tail);
        let cut = hb.len() - 5;
        let mut inp = f(HEADERS, ES, 1, &hb[..cut]);
        inp.extend(f(CONT, EH, 1, &hb[cut..]));
        srv_case(&format!("malformed split {nm}"), server::Builder::new(), inp, 200).await;
    }
    // Pseudo after field
    let mut hb = block(&[(":method", "GET"), (":scheme", "https"), ("x-a", "b"), (":path", "/"), (":authority", "a.test")]);
    hb.extend(lit("x-tail", "value-long-value"));
    let cut = hb.len() - 5;
    let mut inp = f(HEADERS, ES, 1, &hb[..cut]);
    inp.extend(f(CONT, EH, 1, &hb[cut..]));
    srv_case("malformed split pseudo-after-field", server::Builder::new(), inp, 200).await;
}
#[tokio::test]
async fn invalid_fields_conn() {
    for (nm, bad) in [
        ("newline", req(&[("x-a", "a\nb")])),
        ("upper", req(&[("X-Up", "a")])),
        ("unknown pseudo", req(&[(":foo", "a")])),
        ("bad method", block(&[(":method", "G ET"), (":scheme", "https"), (":path", "/")])),
    ] {
        let mut inp = f(HEADERS, EH, 1, &req(&[]));
        inp.extend(f(HEADERS, EH | ES, 3, &bad));
        srv_case(&format!("invalid {nm}"), server::Builder::new(), inp, 200).await;
    }
}
#[tokio::test]
async fn missing_pseudo() {
    for (nm, b) in [
        ("GET no path no auth", block(&[(":method", "GET"), (":scheme", "https")])),
        ("GET no path w/ auth", block(&[(":method", "GET"), (":scheme", "https"), (":authority", "a.test")])),
        ("CONNECT no auth", block(&[(":method", "CONNECT")])),
        ("GET path *", block(&[(":method", "GET"), (":scheme", "https"), (":path", "*"), (":authority", "a.test")])),
        ("GET no scheme", block(&[(":method", "GET"), (":path", "/"), (":authority", "a.test")])),
        ("no method", block(&[(":scheme", "https"), (":path", "/"), (":authority", "a.test")])),
        ("ws value", req(&[("x-a", " v ")])),
    ] {
        srv_case(&format!("pseudo: {nm}"), server::Builder::new(), f(HEADERS, EH | ES, 1, &b), 150).await;
    }
}
#[tokio::test]
async fn content_length() {
    let mut inp = f(HEADERS, EH, 1, &req(&[("content-length", "5"), ("content-length", "100")]));
    inp.extend(f(DATA, ES, 1, b"12345"));
    srv_case("cl 5,100", server::Builder::new(), inp, 150).await;
    let mut inp = f(HEADERS, EH, 1, &req(&[("content-length", "0"), ("content-length", "100")]));
    inp.extend(f(DATA, ES, 1, b""));
    srv_case("cl 0,100", server::Builder::new(), inp, 150).await;
    srv_case("cl empty", server::Builder::new(), f(HEADERS, EH | ES, 1, &req(&[("content-length", "")])), 150).await;
}
#[tokio::test]
async fn list_limit() {
    for lim in [171u32, 172, 173] {
        let mut b = server::Builder::new();
        b.max_header_list_size(lim);
        srv_case(&format!("limit {lim} list 172"), b, f(HEADERS, EH | ES, 1, &req(&[])), 150).await;
    }
}
#[tokio::test]
async fn size_update_midblock() {
    let mut h = lit(":method", "GET");
    h.push(0x20 | 0);
    h.extend(block(&[(":scheme", "https"), (":path", "/"), (":authority", "a.test")]));
    srv_case("size update mid single frame", server::Builder::new(), f(HEADERS, EH | ES, 1, &h), 150).await;
    let h1 = lit(":method", "GET");
    let mut h2b = vec![0x20u8];
    h2b.extend(block(&[(":scheme", "https"), (":path", "/"), (":authority", "a.test")]));
    let mut inp = f(HEADERS, ES, 1, &h1);
    inp.extend(f(CONT, EH, 1, &h2b));
    srv_case("size update at CONTINUATION start", server::Builder::new(), inp, 150).await;
}
#[tokio::test]
async fn self_dep_desync() {
    let mut p = vec![0, 0, 0, 1, 16];
    p.extend(req(&[]));
    p.extend(liti("x-a", "1"));
    let mut inp = f(HEADERS, EH | ES | PRIO, 1, &p);
    let mut b = req(&[]);
    b.push(0x80 | 62);
    inp.extend(f(HEADERS, EH | ES, 3, &b));
    srv_case("self dep desync", server::Builder::new(), inp, 150).await;
    // control
    let mut p = vec![0, 0, 0, 0, 16];
    p.extend(req(&[]));
    p.extend(liti("x-a", "1"));
    let mut inp = f(HEADERS, EH | ES | PRIO, 1, &p);
    let mut b = req(&[]);
    b.push(0x80 | 62);
    inp.extend(f(HEADERS, EH | ES, 3, &b));
    srv_case("valid dep control", server::Builder::new(), inp, 150).await;
}
#[tokio::test]
async fn empty_name() {
    srv_case("empty name single frame", server::Builder::new(), f(HEADERS, EH | ES, 1, &req(&[("", "v")])), 150).await;
    let h = req(&[("", "v")]);
    let mut inp = f(HEADERS, ES, 1, &h);
    inp.extend(f(CONT, EH, 1, &lit("x-b", "c")));
    srv_case("empty name not final frame", server::Builder::new(), inp, 150).await;
}
#[tokio::test]
async fn err_codes() {
    let hb = f(HEADERS, EH, 1, &req(&[]));
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("ping size", f(6, 0, 0, &[0; 7])),
        ("settings size", f(4, 0, 0, &[0; 5])),
        ("settings ack size", f(4, 1, 0, &[0; 6])),
        ("rst size", [hb.clone(), f(3, 0, 1, &[0; 3])].concat()),
        ("wu size", f(8, 0, 0, &[0; 3])),
        ("prio size", f(2, 0, 1, &[0; 3])),
        ("init window 2^31", f(4, 0, 0, &[0, 4, 0x80, 0, 0, 0])),
        ("hpack bad idx", f(HEADERS, EH, 1, &[0x80 | 0x7f, 0x7f])),
        ("zero WU stream", [hb.clone(), wu(1, 0)].concat()),
        ("data after end", [f(HEADERS, EH | ES, 1, &req(&[])), f(DATA, 0, 1, b"x")].concat()),
        ("goaway nonzero", goaway(5, 0, 0)),
    ];
    for (n, i) in cases {
        srv_case(&format!("code {n}"), server::Builder::new(), i, 100).await;
    }
}
#[tokio::test]
async fn forgotten_trailers() {
    let mut b = server::Builder::new();
    b.max_concurrent_reset_streams(0);
    // malformed request gets RST by library -> forgotten; 
    let mut inp = f(HEADERS, EH, 1, &req(&[("connection", "close")]));
    inp.extend(f(DATA, 0, 1, b"x"));
    srv_case("forgot data", b, inp, 150).await;
    let mut b = server::Builder::new();
    b.max_concurrent_reset_streams(0);
    let mut inp = f(HEADERS, EH, 1, &req(&[("connection", "close")]));
    inp.extend(f(HEADERS, EH | ES, 1, &lit("x-t", "v")));
    srv_case("forgot trailers", b, inp, 150).await;
    // Default with reset still remembered
    let mut inp = f(HEADERS, EH, 1, &req(&[("connection", "close")]));
    inp.extend(f(HEADERS, EH | ES, 1, &lit("x-t", "v")));
    srv_case("remembered trailers (control)", server::Builder::new(), inp, 150).await;
}
#[tokio::test]
async fn forgotten_trailers_default_timing() {
    let (io, mut c) = mock::new();
    let l = log();
    let j = spawn_srv(server::Builder::new(), io, l.clone());
    c.assert_server_handshake().await;
    c.send_bytes(&f(HEADERS, EH, 1, &req(&[("connection", "close")]))).await;
    let a = dump(&mut c, 50).await;
    sleep(Duration::from_millis(1100)).await;
    // something to make the conn clear expired
    c.send_bytes(&f(6, 0, 0, &[0; 8])).await;
    let _b = dump(&mut c, 100).await;
    c.send_bytes(&f(HEADERS, EH | ES, 1, &lit("x-t", "v"))).await;
    let fr = dump(&mut c, 200).await;
    show("forgotten trailers default timing", &[a, fr].concat(), &l);
    j.abort();
}
#[tokio::test]
async fn empty_data_memory_shape() {
    // shape only: 2000 empty DATA frames on an unread body are accepted w/out error
    let (io, mut c) = mock::new();
    let j = tokio::spawn(async move {
        let mut conn = server::handshake(io).await.unwrap();
        let (_req, _sr) = conn.accept().await.unwrap().unwrap();
        while let Some(_) = conn.accept().await {}
        drop(_req);
    });
    c.assert_server_handshake().await;
    let mut inp = f(HEADERS, EH, 1, &req(&[]));
    for _ in 0..2000 {
        inp.extend(f(DATA, 0, 1, b""));
    }
    c.send_bytes(&inp).await;
    let fr = dump(&mut c, 200).await;
    println!("empty data frames: {fr:#?}");
    j.abort();
}
