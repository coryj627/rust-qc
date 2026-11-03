use serde::Deserialize;
fn main(){
  let mut buf=b"1".to_vec();
  let mut de=simd_json::Deserializer::from_slice(&mut buf).unwrap();
  println!("{:?}",u8::deserialize(&mut de).map_err(|e|e.to_string()));
  println!("{:?}",u8::deserialize(&mut de).map_err(|e|e.to_string()));
}
