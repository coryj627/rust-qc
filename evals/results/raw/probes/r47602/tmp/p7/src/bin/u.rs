use simd_json::prelude::*;
fn main(){
    let mut b = b"\"a\xffb\"".to_vec();
    match simd_json::to_owned_value(&mut b) {
        Ok(v)=>{ let s=v.as_str().unwrap(); println!("ACCEPTED invalid utf8: bytes={:?} std valid={}", s.as_bytes(), std::str::from_utf8(s.as_bytes()).is_ok()); }
        Err(e)=>println!("rejected {e}"),
    }
    let mut b = br#"     {"abc": "xyz\q"}"#.to_vec();
    println!("{:?}", simd_json::to_owned_value(&mut b).map_err(|e|(e.index(),e.character())).is_ok());
    let mut b = br#"     {"abc": "xyz\q"}"#.to_vec();
    let e=simd_json::to_owned_value(&mut b).unwrap_err(); println!("native escape err index={} ch={:?}", e.index(), e.character());
    let mut b = br#""\ud83d""#.to_vec();
    println!("{:?}", simd_json::to_owned_value(&mut b).map(|v| v.encode()).map_err(|e|e.to_string()));
}
