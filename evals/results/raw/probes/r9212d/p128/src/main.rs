use simd_json::prelude::*;
fn main(){
 let mut b=b"18446744073709551616".to_vec(); println!("f64 from u128 {:?}",simd_json::from_slice::<f64>(&mut b).map_err(|e|e.to_string()));
 let mut b=b"-18446744073709551616".to_vec(); println!("f64 from i128 {:?}",simd_json::from_slice::<f64>(&mut b).map_err(|e|e.to_string()));
 let mut b=b"[18446744073709551616abc]".to_vec(); println!("garbage {:?}",simd_json::to_owned_value(&mut b).map_err(|e|e.to_string()));
 // known-key over the 32-key vec threshold
 let mut s=String::from("{"); for i in 0..100 { if i>0 {s.push(',');} s+=&format!("\"k{i}\":{i}"); } s.push('}'); let mut b=s.into_bytes();
 let v=simd_json::to_borrowed_value(&mut b).unwrap(); for k in ["k0","k50","k99","nope"] { let kk=simd_json::KnownKey::from(k); println!("{k}: {:?} / get {:?}",kk.lookup(&v),v.get(k)); }
 let mut bv:simd_json::BorrowedValue=simd_json::json!({"a":1}).into(); let kk=simd_json::KnownKey::from("b"); 
 println!("{:?}",bv.encode());
 let lazy_v=simd_json::BorrowedValue::from(1); let kk=simd_json::KnownKey::from("c"); println!("nonobj lookup {:?}",kk.lookup(&lazy_v));
}
