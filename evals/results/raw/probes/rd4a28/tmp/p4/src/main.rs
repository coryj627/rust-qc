use simd_json::prelude::*;
use serde::Deserialize;
fn main(){
 let args:Vec<String>=std::env::args().collect();
 match args.get(1).map(|s|s.as_str()).unwrap_or("") {
  "ub1"=>{ let t=simd_json::Tape::null().reset(); println!("{:?}", t.deserialize::<u64>().map_err(|e|e.to_string())); }
  "ub2"=>{ let mut b=b"1".to_vec(); let mut d=simd_json::Deserializer::from_slice(&mut b).unwrap(); let a=u64::deserialize(&mut d); let c=u64::deserialize(&mut d); println!("{a:?} {c:?}"); }
  "ub4"=>{ let t=simd_json::Tape(vec![simd_json::Node::Array{len:1,count:0}]); let l=simd_json::value::lazy::Value::from_tape(t.as_value()); println!("{:?}", l.into_value()); }
  _=>{
   // aliasing: strings retained, escapes rewriting input in place, buffers reused, tape, borrowed, serde borrowed
   let src=br#"{"a\n":"first","b":["second\tx","@u00e9@ud83d@ude00 third",{"k":"plain","k\"2":"esc\\aped"}],"long string over thirty-two bytes, to cross chunks":"yes it is long enough to cross the chunk boundary \" and escape","n":[1,2.5,-3,1e10,12345678901234567890,1.2345678901234567890123,true,false,null]}"#.to_vec(); let src:Vec<u8>=src.iter().map(|c| if *c==b'@' {b'\\'} else {*c}).collect();
   let mut a=src.clone(); let t=simd_json::to_tape(&mut a).unwrap(); let v=t.as_value(); let o=v.as_object().unwrap(); let keys:Vec<&str>=o.keys().collect(); println!("{keys:?}"); println!("{}", v.encode().len());
   let mut a=src.clone(); let bv=simd_json::to_borrowed_value(&mut a).unwrap(); println!("{}", bv.encode().len()); let st=bv.clone().into_static(); drop(bv); println!("{}", st.encode().len());
   let mut a=src.clone(); let ov=simd_json::to_owned_value(&mut a).unwrap(); println!("{}", ov.encode_pp().len());
   let mut buf=simd_json::Buffers::new(4); for _ in 0..3 { let mut a=src.clone(); let v=simd_json::to_owned_value_with_buffers(&mut a,&mut buf).unwrap(); println!("{}", v.encode().len()); let mut s=b"[1,\"x\"]".to_vec(); let v=simd_json::to_borrowed_value_with_buffers(&mut s,&mut buf).unwrap(); println!("{}", v.encode()); }
   let mut a=src.clone(); let mut t=simd_json::Tape::null(); let mut a2=src.clone(); simd_json::fill_tape(&mut a,&mut buf,&mut t).unwrap(); println!("{}", t.as_value().encode().len()); let t2=t.reset(); let mut t2=t2; simd_json::fill_tape(&mut a2,&mut buf,&mut t2).unwrap(); println!("{}", t2.as_value().encode().len());
   #[derive(Deserialize,Debug)] struct S<'a>{ #[serde(borrow)] a:&'a str, #[serde(borrow)] b:Vec<&'a str>}
   let mut a:Vec<u8>=br#"{"a":"x\ny","b":["q","r\"s","@u00e9"]}"#.iter().map(|c| if *c==b'@' {b'\\'} else {*c}).collect(); println!("{:?}", simd_json::from_slice::<S>(&mut a).unwrap());
   let mut bad=br#"{"a":"x\ny","b":["q","\ud800"],"c":"\q"}"#.to_vec(); println!("{:?}", simd_json::to_tape(&mut bad).map(|_|()).map_err(|e|e.to_string()));
   let l=simd_json::value::lazy::Value::from_tape({ let t:&'static simd_json::Tape<'static>=Box::leak(Box::new({ let m:&'static mut Vec<u8>=Box::leak(Box::new(src.clone())); simd_json::to_tape(m).unwrap()})); t.as_value()}); println!("{}", l.is_tape()); let mut l2=l.clone(); l2.insert("z",1).ok(); println!("{}", l2.is_value()); println!("{}", l.into_value().encode().len());
  }
 }
}
