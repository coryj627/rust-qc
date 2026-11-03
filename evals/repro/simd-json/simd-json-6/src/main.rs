// Number parsing with exponents beyond 32 bits. Correct results follow RFC 8259 sec. 6
// (no limit on the exponent; 0eN is zero) and the crate's own treatment of 1e309 (out of
// range is an error).
fn p(s: &str) -> Result<simd_json::OwnedValue, simd_json::Error> {
    let mut d = s.as_bytes().to_vec();
    simd_json::to_owned_value(&mut d)
}
fn main() {
    let mut bad = 0;
    println!("1e309 -> {:?} (the crate's handling of out-of-range)", p("1e309"));
    println!("serde_json 1e4294967296 -> {:?}", serde_json::from_str::<f64>("1e4294967296").map_err(|e| e.to_string()));
    println!("serde_json 0e99999999999 -> {:?}", serde_json::from_str::<f64>("0e99999999999").map_err(|e| e.to_string()));
    for (s, wrong) in [("1e4294967296", 1.0f64), ("1e4294967295", 0.1), ("1e-4294967295", 10.0), ("1.5e-8589934592", 1.5)] {
        let r = p(s);
        println!("{s} -> {r:?}");
        if let Ok(simd_json::OwnedValue::Static(simd_json::StaticNode::F64(f))) = r {
            if f == wrong { bad += 1; }
        }
    }
    for s in ["0e99999999999", "1e-99999999999"] {
        let r = p(s);
        println!("{s} -> {r:?}");
        if r.is_err() { bad += 1; }
    }
    println!("wrong_results={bad}");
    std::process::exit(if bad > 0 { 42 } else { 0 });
}
