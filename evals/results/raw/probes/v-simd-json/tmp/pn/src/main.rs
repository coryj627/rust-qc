fn main(){ let mut b=b"\"a\xffb\"".to_vec(); let v=simd_json::to_borrowed_value(&mut b); match v { Ok(simd_json::BorrowedValue::String(s))=>println!("Ok bytes {:?}", s.as_bytes()), o=>println!("{:?}",o)}; 
 println!("alg {:?}", simd_json::Deserializer::algorithm().to_string());
 let mut b=br#"            "abc\q""#.to_vec(); println!("{:?}", simd_json::to_owned_value(&mut b).map_err(|e|e.index())); }
