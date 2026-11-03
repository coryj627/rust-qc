use simd_json::prelude::*;
use serde::Deserialize;
fn deep(n: usize) -> Vec<u8> { let mut v = vec![b'['; n]; v.extend(vec![b']'; n]); v }
fn main(){
 let args:Vec<String>=std::env::args().collect();
 match args.get(1).map(|s|s.as_str()).unwrap_or("") {
  "m"=>{ let n:usize=args[2].parse().unwrap(); let mut b=deep(n); let v=simd_json::to_owned_value(&mut b).unwrap(); std::mem::forget(v); println!("ok"); }
  "t"=>{ let n:usize=args[2].parse().unwrap(); std::thread::Builder::new().stack_size(2<<20).spawn(move||{ let mut b=deep(n); let v=simd_json::to_owned_value(&mut b).unwrap(); std::mem::forget(v); println!("ok"); }).unwrap().join().unwrap(); }
  "tape"=>{ let n:usize=args[2].parse().unwrap(); let mut b=deep(n); let v=simd_json::to_tape(&mut b).unwrap(); println!("to_tape ok {}", v.0.len()); std::io::Write::flush(&mut std::io::stdout()).ok(); println!("enc {}", v.as_value().encode().len()); }
  _=>{
   let b5='\x5c';
   let key=format!("{{\"a{b5}n\":1,\"plain\":\"v{b5}n\"}}");
   let mut d=serde_json::Deserializer::from_str(&key); println!("BorrowedValue from serde_json escaped key: {:?}", simd_json::BorrowedValue::deserialize(&mut d).map(|v|v.encode()).map_err(|e|e.to_string()));
   let mut d=serde_json::Deserializer::from_str(r#"{"a":1,"plain":"x"}"#); println!("BorrowedValue from serde_json plain: {:?}", simd_json::BorrowedValue::deserialize(&mut d).map(|v|v.encode()).map_err(|e|e.to_string()));
   let mut d=serde_json::Deserializer::from_str(&key); println!("OwnedValue from serde_json escaped key: {:?}", simd_json::OwnedValue::deserialize(&mut d).map(|v|v.encode()).map_err(|e|e.to_string()));
   let ov=simd_json::json!({"a":1}); let r:Result<simd_json::BorrowedValue,_>=simd_json::BorrowedValue::deserialize(&ov); println!("BorrowedValue from &OwnedValue {:?}", r.map(|v|v.encode()).map_err(|e|e.to_string()));
   let mut s=key.clone().into_bytes(); println!("BorrowedValue from_slice escaped key {:?}", simd_json::from_slice::<simd_json::BorrowedValue>(&mut s).map(|v|v.encode()).map_err(|e|e.to_string()));
   let mut s=b"{\"a\":[1,{\"b\":null}],\"c\":\"str\"}".to_vec(); let v=simd_json::to_owned_value(&mut s).unwrap(); println!("Display owned: {v}"); println!("Display str: {}", simd_json::OwnedValue::from("a\"b"));
   let mut s=b"[1,-1,2.5,true,null,\"s\"]".to_vec(); let t=simd_json::to_tape(&mut s).unwrap(); let tv=t.as_value(); for x in tv.as_array().unwrap().iter(){ print!("{:?}/{:?}/{} ", x.cast_f64(), x.as_f64(), x.is_f64_castable()); } println!(); 
   let mut s=b"[1,-1,2.5,true,null,\"s\"]".to_vec(); let o=simd_json::to_owned_value(&mut s).unwrap(); for x in o.as_array().unwrap().iter(){ print!("{:?}/{:?}/{} ", x.cast_f64(), x.as_f64(), x.is_f64_castable()); } println!();
   // realistic float docs
   for doc in [r#"{"id":"sensor-42","site":"north-field","reading":3.14159265358979323846264338}"#, r#"{"name":"pi","value":3.14159265358979323846,"note":"thirty digits"}"#, r#"{"a":1,"b":2,"c":3,"d":4,"e":5,"f":6,"g":7,"h":8,"lat":47.60357000000000000000012}"#, r#"[0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,1.1,1.2,1.3,123456789012345678901.5]"#] { let mut s=doc.as_bytes().to_vec(); println!("{doc}\n  -> {:?}", simd_json::to_owned_value(&mut s).map(|v|v.encode()).map_err(|e|e.to_string())); println!("  sj {:?}", serde_json::from_str::<serde_json::Value>(doc).map(|v|v.to_string()).map_err(|e|e.to_string())); }
   // lazy
   let mut src=br#"{"a":[1,2,3],"o":{"x":1,"y":[5,6]},"s":"str","n":1,"f":1.5}"#.to_vec(); let t=simd_json::to_tape(&mut src).unwrap(); let l=simd_json::value::lazy::Value::from_tape(t.as_value());
   let mut l2=l.clone(); l2.insert("zz",1).unwrap(); println!("lazy tape/value is_tape {} {}", l.is_tape(), l2.is_tape());
   for (n,lv) in [("tape",&l),("val",&l2)] { let a=lv.as_array(); println!("{n} as_array root {:?}", a.map(|a|a.len())); let g=lv.get("a").unwrap(); let arr=g.as_array().unwrap(); println!(" {n} a.len={} iter={:?} get1={:?}", arr.len(), arr.iter().map(|v|v.as_u64()).collect::<Vec<_>>(), arr.get(1).map(|v|v.as_u64())); let o=lv.as_object().unwrap(); println!(" {n} o.len={} keys={:?}", o.len(), { let mut k:Vec<_>=o.keys().collect(); k.sort(); k}); println!(" {n} get_u64 n={:?} get_str s={:?} get_f64 f={:?} contains={}", lv.get_u64("n"), lv.get_str("s"), lv.get_f64("f"), lv.contains_key("s")); println!(" {n} encode {}", lv.encode().len()); }
   // buffers reuse
   let mut buf=simd_json::Buffers::new(0); for n in [0usize,1,63,64,65,127,128,129,1000,10,100000,5] { let mut d=format!("[\"{}\",1,{{\"a\":\"b\"}}]","x".repeat(n)).into_bytes(); let v=simd_json::to_owned_value_with_buffers(&mut d,&mut buf).unwrap(); assert_eq!(v[0].as_str().unwrap().len(),n); } println!("buffers ok");
   let mut bad=b"[1,".to_vec(); println!("buf err {:?}", simd_json::to_owned_value_with_buffers(&mut bad,&mut buf).map(|_|()).map_err(|e|e.to_string()));
   // restart + deserializer reuse
   let mut x=b"[1,2]".to_vec(); let mut d=simd_json::Deserializer::from_slice(&mut x).unwrap(); let a=<Vec<u8>>::deserialize(&mut d).unwrap(); d.restart(); let c=<Vec<u8>>::deserialize(&mut d).unwrap(); println!("restart {a:?} {c:?} as_value {}", d.as_value().encode());
   // writer error
   struct W; impl std::io::Write for W { fn write(&mut self,_:&[u8])->std::io::Result<usize>{Err(std::io::Error::other("nope"))} fn flush(&mut self)->std::io::Result<()>{Ok(())} }
   let e=simd_json::to_writer(W,&vec![1]).unwrap_err(); println!("writer err {} io={} src={:?}", e, e.is_io(), std::error::Error::source(&e).is_some());
   struct R; impl std::io::Read for R { fn read(&mut self,_:&mut [u8])->std::io::Result<usize>{Err(std::io::Error::other("nope"))} } let e=simd_json::from_reader::<_,Vec<u8>>(R).unwrap_err(); println!("reader err {} io={}", e, e.is_io());
   let mut e=b"{\"a\":1".to_vec(); let er=simd_json::to_owned_value(&mut e).unwrap_err(); println!("Err idx {} char {:?}", er.index(), er.character());
   let mut cases=vec![b"[\"\xff\"]".to_vec(), b"[\"\t\"]".to_vec(), b"[1,2,\"abc".to_vec()]; for c in cases.iter_mut(){ let er=simd_json::to_owned_value(c).unwrap_err(); println!("stage1 err {:?} idx {} char {:?}", er.error(), er.index(), er.character()); }
   // json macro
   let v=simd_json::json!({"a":[1,2.5,null,true],"b":{"c":"d"},"e":-1}); println!("json! {}", v.encode());
   // eq
   let a=simd_json::OwnedValue::from(1u64); let b=simd_json::OwnedValue::from(1i64); println!("eq U64/I64 {} f {}", a==b, a==simd_json::OwnedValue::from(1.0));
   let mut p=b"{\"a\":1,\"b\":2}".to_vec(); let mut q=b"{\"b\":2,\"a\":1}".to_vec(); let (pv,qv)=(simd_json::to_owned_value(&mut p).unwrap(),simd_json::to_owned_value(&mut q).unwrap()); println!("order-insensitive eq {}", pv==qv);
   let mut p=b"{\"a\":1,\"b\":2}".to_vec(); let mut q=b"{\"b\":2,\"a\":1}".to_vec(); let (pt,qt)=(simd_json::to_tape(&mut p).unwrap(),simd_json::to_tape(&mut q).unwrap()); println!("tape eq reorder {}", pt.as_value()==qt.as_value());
   let mut p=b"{\"a\":1,\"b\":2}".to_vec(); let t=simd_json::to_tape(&mut p).unwrap(); let bb=simd_json::to_borrowed_value(&mut b"{\"a\":1,\"b\":2}".to_vec()).map(|v|v.into_static()).unwrap(); let lazy=simd_json::value::lazy::Value::from_tape(t.as_value()); println!("lazy into_value eq {}", lazy.into_value()==bb);
   let mut u=b"1".to_vec(); println!("tape eq num {}", simd_json::to_tape(&mut u).unwrap().as_value()==1u64);
  }
 }
}
