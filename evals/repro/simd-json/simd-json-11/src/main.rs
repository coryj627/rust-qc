// RFC 8259 sec. 2: the only whitespace is space, tab, LF, CR. NUL is not valid after a number.
fn main() {
    let mut bad = 0;
    for s in [&b"123\0"[..], b"[123\0]", b"-1.5e3\0", b"[-1.5e3\0]", b"[-2374611873366417043\0]"] {
        let mut d = s.to_vec();
        let r = simd_json::to_owned_value(&mut d);
        let sj = serde_json::from_slice::<serde_json::Value>(s).map_err(|e| e.to_string());
        println!("{:?}: simd-json={r:?}; serde_json={sj:?}", String::from_utf8_lossy(s));
        if r.is_ok() { bad += 1; }
    }
    // Control: NUL after a string is rejected by the same parser.
    let mut d = b"[\"snot\"\0]".to_vec();
    println!("after string: {:?}", simd_json::to_owned_value(&mut d));
    println!("accepted={bad}");
    std::process::exit(if bad > 0 { 42 } else { 0 });
}
