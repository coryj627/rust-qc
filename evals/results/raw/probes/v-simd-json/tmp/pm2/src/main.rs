#![allow(unused)]
use serde::Deserialize;
use simd_json::prelude::*;
fn main(){
 let a=std::env::args().nth(1).unwrap_or("strs".into());
 match a.as_str() {
  "strs"=>{let mut b=br#"["ab","cd"]"#.to_vec(); let t=simd_json::to_tape(&mut b).unwrap(); println!("{:?}",t.0);
           let v=t.as_value(); println!("{:?}", v.as_array().unwrap().iter().map(|x|x.as_str().map(|s|s.len())).collect::<Vec<_>>());}
  "dbl"=>{let mut b=b"1".to_vec(); let mut d=simd_json::Deserializer::from_slice(&mut b).unwrap(); let _=u8::deserialize(&mut d); println!("{:?}",u8::deserialize(&mut d).is_ok());}
  "reset"=>{println!("{:?}",simd_json::tape::Tape::null().reset().deserialize::<u8>().is_ok());}
  "lazy"=>{use simd_json::tape::{Tape,Node}; let t=Tape(vec![Node::Array{len:3,count:3}]); let v=simd_json::value::lazy::Value::from_tape(t.as_value()).into_value(); println!("{:?}",v);}
  "dup"=>{let mut s=String::from("{"); for i in 0..100 {s+="\"k\":1,";} s+="\"k\":2}"; let mut b=s.into_bytes(); let mut v=simd_json::to_owned_value(&mut b).unwrap(); let m=v.as_object_mut().unwrap(); println!("{}",m.len()); while m.remove("k").is_some(){} println!("{}",m.len());}
  "big"=>{let mut s=String::from("{"); for i in 0..40 {s+=&format!("\"k{i}\":1,"); s+=&format!("\"k{i}\":2,");} s+="\"z\":2}"; let mut b=s.into_bytes(); let mut v=simd_json::to_owned_value(&mut b).unwrap(); let m=v.as_object_mut().unwrap(); println!("{}",m.len()); for i in 0..40 { while m.remove(&format!("k{i}")).is_some(){} } println!("{}",m.len()); }
  _=>panic!()
 }
}
