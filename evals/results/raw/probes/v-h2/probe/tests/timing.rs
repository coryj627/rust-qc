use probe::*;
// huffman: 'a' = 00011 (5 bits)
fn huff_a(n: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(n * 5 / 8 + 1);
    let (mut acc, mut bits) = (0u32, 0);
    for _ in 0..n {
        acc = (acc << 5) | 0b00011; bits += 5;
        while bits >= 8 { bits -= 8; out.push((acc >> bits) as u8); acc &= (1 << bits) - 1; }
    }
    if bits > 0 { out.push(((acc << (8 - bits)) | ((1 << (8 - bits)) - 1)) as u8); }
    out
}
async fn case(name_chars: usize, value_frames: usize) {
    let (io, mut c) = mock::new();
    let t = Arc::new(Mutex::new(None));
    let t2 = t.clone();
    let j = tokio::spawn(async move {
        let mut b = server::Builder::new();
        b.max_frame_size(16384);
        let mut conn = b.handshake::<_, Bytes>(io).await.unwrap();
        let r = conn.accept().await;
        *t2.lock().unwrap() = Some(Instant::now());
        println!("  accept: {:?}", r.map(|x| x.map(|_| ())));
    });
    c.assert_server_handshake().await;
    let name = huff_a(name_chars);
    let mut hb = req(&[]);
    hb.push(0);
    hb.extend(hint(0x80, 7, name.len()));
    hb.extend(&name);
    let vlen = value_frames * 16000;
    hb.extend(hint(0, 7, vlen));
    let name_end = hb.len();
    let chunks: Vec<&[u8]> = hb.chunks(16384).collect();
    let mut wire = vec![];
    for (i, ch) in chunks.iter().enumerate() {
        wire.extend(f(if i == 0 { HEADERS } else { CONT }, if i == 0 { ES } else { 0 }, 1, ch));
    }
    let _ = name_end;
    c.send_bytes(&wire).await;
    sleep(Duration::from_millis(50)).await;
    let val = vec![b'v'; 16000];
    let t0 = Instant::now();
    let mut wire = vec![];
    for i in 0..value_frames {
        wire.extend(f(CONT, if i == value_frames - 1 { EH } else { 0 }, 1, &val));
    }
    c.send_bytes(&wire).await;
    let _ = j.await;
    let end = t.lock().unwrap().unwrap();
    println!("name {name_chars} chars ({} B huffman), {value_frames} value frames: {:?}, total block {} KB", name.len(), end.duration_since(t0), (hb.len() + vlen) / 1024);
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redecode() {
    for (n, k) in [(1_000_000, 1), (1_000_000, 50), (1_000_000, 100), (4_000_000, 100), (13_000_000, 490)] {
        case(n, k).await;
    }
}
