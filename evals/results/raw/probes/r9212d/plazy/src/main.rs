use simd_json::prelude::*; use simd_json::value::lazy::Value as L;
fn main(){
 let mut a=br#"{"a":1}"#.to_vec(); let t=simd_json::to_tape(&mut a).unwrap(); let tv=t.as_value();
 let l1=L::from_tape(tv); let mut l2=L::from_tape(tv); let _=l2.insert("a",1); println!("tape-lazy {} vs value-lazy {} equal={}",l1.encode(),l2.encode(),l1==l2);
 let mut l3=L::from_tape(tv); println!("insert prev {:?} remove {:?} push-non-array {:?}",l3.insert("x",2).map(|o|o.is_some()),l3.remove("x").map(|o|o.is_some()),l3.push(1).is_err());
 let mut c=br#"{"k":[1,2,3],"o":{"a":1,"b":2}}"#.to_vec(); let t2=simd_json::to_tape(&mut c).unwrap(); let l=L::from_tape(t2.as_value());
 println!("lazy get_array iter {:?}",l.get_array("k").map(|a|a.iter().map(|x|x.as_u64()).collect::<Vec<_>>()));
 println!("lazy get_object b {:?}",l.get_object("o").map(|o|o.get("b").map(|x|x.as_u64())));
 println!("tape display {}",l); println!("owned display arr {}",simd_json::json!([1,"a"]));
 println!("borrowed display {}",simd_json::BorrowedValue::from(simd_json::json!({"a":"x"})));
 let e=simd_json::to_owned_value(&mut b"truex".to_vec()).unwrap_err(); println!("truex is_syntax {} is_data {}",e.is_syntax(),e.is_data());
 let e=simd_json::to_owned_value(&mut b"[] x".to_vec()).unwrap_err(); println!("[] x {:?}",e.error());
 let e=simd_json::to_owned_value(&mut b"".to_vec()).unwrap_err(); println!("empty {:?} eof {}",e.error(),e.is_eof());
 let e=simd_json::to_owned_value(&mut b"[1,".to_vec()).unwrap_err(); println!("[1, {:?} eof {} syn {}",e.error(),e.is_eof(),e.is_syntax());
 let e=simd_json::to_owned_value(&mut b"\"abc".to_vec()).unwrap_err(); println!("unterminated {:?} eof {} syn {}",e.error(),e.is_eof(),e.is_syntax());
 use std::error::Error as _; let ioe=simd_json::Error::generic(simd_json::ErrorType::Io(std::io::Error::other("x"))); println!("source {:?}",ioe.source().is_some());
 let huge=std::panic::catch_unwind(||simd_json::Buffers::new(usize::MAX)); println!("Buffers::new(MAX) panic={}",huge.is_err());
 let huge=std::panic::catch_unwind(||simd_json::Buffers::new(usize::MAX-20)); println!("Buffers::new(MAX-20) panic={}",huge.is_err());
}
