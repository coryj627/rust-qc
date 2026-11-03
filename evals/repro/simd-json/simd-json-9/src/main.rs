// Any parse through the NEON stage 1 from safe code.
fn main() {
    let mut d = br#"{"a":[1,2,3],"b":"c"}"#.to_vec();
    let v = simd_json::to_owned_value(&mut d);
    println!("parsed ok={}", v.is_ok());
}
