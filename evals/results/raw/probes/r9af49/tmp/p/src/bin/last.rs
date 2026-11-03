use serde::{Serialize,Serializer};
use simd_json::prelude::*;
struct Fil(Vec<i32>);
impl Serialize for Fil { fn serialize<S:Serializer>(&self,s:S)->Result<S::Ok,S::Error>{ s.collect_seq(self.0.iter().filter(|x|**x>100)) } }
struct FilM(Vec<i32>);
impl Serialize for FilM { fn serialize<S:Serializer>(&self,s:S)->Result<S::Ok,S::Error>{ s.collect_map(self.0.iter().filter(|x|**x>100).map(|x|(x.to_string(),*x))) } }
#[derive(Serialize)] struct Wrap{items:Fil,next:i32}
fn main(){
  println!("compact collect_seq empty filter {:?}  sj {:?}",simd_json::to_string(&Wrap{items:Fil(vec![1]),next:2}),serde_json::to_string(&Wrap{items:Fil(vec![1]),next:2}).unwrap());
  println!("pretty  collect_seq empty filter {:?}",simd_json::to_string_pretty(&Fil(vec![1])));
  println!("compact collect_map empty filter {:?}",simd_json::to_string(&FilM(vec![1])));
  println!("pretty  collect_map empty filter {:?}",simd_json::to_string_pretty(&FilM(vec![1])));
  for s in ["[1\0garbage]","1\0garbage","true\0garbage","{\"a\":null\0zzz}","[1.5\0x,2]","nullgarbage"] { let mut b=s.as_bytes().to_vec(); println!("{:?} -> {:?}",s,simd_json::to_owned_value(&mut b)); }
  let mut b=br#"{"a":1,"a":2}"#.to_vec(); let v=simd_json::to_owned_value(&mut b).unwrap();
  println!("dup v==v.clone() {} get a {:?}",v==v.clone(),v.get("a"));
  let mut b=br#"{"a":1,"a":2}"#.to_vec(); let w=simd_json::from_slice::<simd_json::OwnedValue>(&mut b).unwrap(); println!("from_slice::<OwnedValue> {:?} get a {:?}",w,w.get("a"));
  let mut b=br#"{"a":1,"a":2}"#.to_vec(); let t=simd_json::to_tape(&mut b).unwrap(); println!("tape get a {:?}",t.as_value().get("a").and_then(|x|x.as_u64()));
  println!("sj {:?}",serde_json::from_str::<serde_json::Value>(r#"{"a":1,"a":2}"#).unwrap());
  let doc=r#"{"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa":0.21575530329307963391}"#;
  let mut b=doc.as_bytes().to_vec(); println!("{} -> {:?}  std {:?}",doc,simd_json::to_owned_value(&mut b),"0.21575530329307963391".parse::<f64>());
  let doc=r#"{"name":"a sample record with a long value","price":12.3456789012345678901234567}"#;
  let mut b=doc.as_bytes().to_vec(); println!("{} -> {:?}",doc,simd_json::to_owned_value(&mut b));
  for p in 40..100 { let doc=format!("{}12.3456789012345678901234567"," ".repeat(p)); let mut b=doc.into_bytes(); let v=simd_json::to_owned_value(&mut b).unwrap(); let f=v.as_f64().unwrap(); if (f-12.345678901234567).abs()>1e-9 {print!("pad {p}={f} ");} } println!();
  let mut b=b"5".to_vec(); let t=simd_json::to_tape(&mut b).unwrap(); println!("tape 5 cast_f64 {:?} as_f64 {:?}",t.as_value().cast_f64(),t.as_value().as_f64());
  let mut b=b"[\"\\ud800\\uffff\",\"\\ud800\\ue000\",\"\\udbff\\uffff\",\"\\ud800\\u0041\"]".to_vec(); println!("{:?}",simd_json::to_owned_value(&mut b).map(|v|v.as_array().unwrap().iter().map(|x|x.as_str().unwrap().chars().map(|c|format!("{:x}",c as u32)).collect::<Vec<_>>()).collect::<Vec<_>>()));
  let mut e=b"nul".to_vec(); let er=simd_json::to_owned_value(&mut e).unwrap_err(); println!("nul: is_syntax {} is_data {}",er.is_syntax(),er.is_data());
}
