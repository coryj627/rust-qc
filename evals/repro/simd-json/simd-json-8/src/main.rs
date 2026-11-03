// A lone high surrogate must be an error, not U+0000.
use simd_json::prelude::*;
fn main() {
    let mut bad = 0;
    for s in [r#""\ud800""#, r#""a\ud800b""#, r#""\ud800\uzzzz""#] {
        let mut d = s.as_bytes().to_vec();
        let r = simd_json::to_owned_value(&mut d);
        let sj = serde_json::from_str::<String>(s).map_err(|e| e.to_string());
        println!("{s}: simd-json to_owned_value={r:?}; serde_json={sj:?}");
        if let Ok(v) = &r { if v.as_str().map_or(false, |x| x.contains('\0')) { bad += 1; } }
        let mut d = s.as_bytes().to_vec();
        let r = simd_json::from_slice::<String>(&mut d);
        println!("  simd-json from_slice::<String>={r:?}");
    }
    // Control: a low surrogate alone is rejected by the same function.
    let mut d = br#""\udc00""#.to_vec();
    println!("lone low surrogate: {:?}", simd_json::to_owned_value(&mut d));
    println!("nul_results={bad}");
    std::process::exit(if bad > 0 { 42 } else { 0 });
}
