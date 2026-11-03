// Safe code only: reuse a public Deserializer past the end of its tape.
use serde::Deserialize;
fn main() {
    let mut d = b"123".to_vec();
    let mut de = simd_json::Deserializer::from_slice(&mut d).unwrap();
    let first = u8::deserialize(&mut de);
    println!("first = {first:?}");
    // Tape has one node. This call must return Err; it reads past the tape.
    let second = u8::deserialize(&mut de);
    println!("second = {second:?}");
    assert!(second.is_err());
    println!("no bug: second call returned Err");
}
