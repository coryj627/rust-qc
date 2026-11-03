fn main(){
 let mut b=vec![b'"',0xff,0xfe,b'"'];
 let r=simd_json::to_borrowed_value(&mut b);
 println!("native-path invalid UTF-8 inside string: {:?}", r.as_ref().map(|v|format!("{:?}",std::mem::discriminant(v))));
 if let Ok(simd_json::BorrowedValue::String(s))=&r { println!("str bytes {:?} is_valid_utf8={}",s.as_bytes(),std::str::from_utf8(s.as_bytes()).is_ok()); }
 let mut b=vec![b'[',b'1',b',',0xff,b']']; println!("{:?}",simd_json::to_owned_value(&mut b));
 let mut b=b"\"\\ud800\"".to_vec(); println!("{:?}",simd_json::to_owned_value(&mut b));
 let mut b=b"             \"abc\\q\"".to_vec(); println!("{:?}",simd_json::to_owned_value(&mut b));
 let mut b=b"\"a\\nb\"".to_vec(); println!("{:?}",simd_json::to_owned_value(&mut b));
}
