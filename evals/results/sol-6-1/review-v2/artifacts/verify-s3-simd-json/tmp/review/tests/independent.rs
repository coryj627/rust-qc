use serde::{Deserialize, Serialize, Serializer};
use serde::ser::{SerializeSeq, SerializeMap};
use simd_json::prelude::*;
use simd_json::{OwnedValue, BorrowedValue};
use std::collections::BTreeMap;

fn owned(s: &str) -> OwnedValue { simd_json::to_owned_value(&mut s.as_bytes().to_vec()).unwrap() }
#[test]
fn decimal_offset() {
 for s in ["12345678901234567890.0", "1.2345678901234567890123456789"] {
  let want: f64 = s.parse().unwrap();
  for offset in [0, 52, 60, 64, 100] {
   let got = owned(&format!("{}{s}", " ".repeat(offset))).as_f64().unwrap();
   println!("decimal {s} offset {offset}: {got:?} expected {want:?}");
  }
 }
 assert_eq!(owned(&format!("{}1.2345678901234567890123456789", " ".repeat(64))).as_f64(), Some("1.2345678901234567890123456789".parse().unwrap()));
}
#[test]
fn tape_containers() {
 for s in [r#"{"a":[1,2]}"#, r#"{"a":[]}"#, r#"{"a":[[1,2]]}"#] {
  let mut b = s.as_bytes().to_vec(); let t = simd_json::to_tape(&mut b).unwrap(); let v = t.as_value();
  for try_variant in [false,true] {
   let a = if try_variant {v.try_get_array("a").unwrap().unwrap()} else {v.get_array("a").unwrap()};
   let result = std::panic::catch_unwind(|| (a.len(), a.iter().map(|v| v.encode()).collect::<Vec<_>>()));
   println!("array {s} try={try_variant}: {result:?}");
  }
 }
 for s in [r#"{"a":{"x":1,"y":2}}"#, r#"{"a":{}}"#] {
  let mut b = s.as_bytes().to_vec(); let t = simd_json::to_tape(&mut b).unwrap(); let v = t.as_value();
  for try_variant in [false,true] {
   let a = if try_variant {v.try_get_object("a").unwrap().unwrap()} else {v.get_object("a").unwrap()};
   let result = std::panic::catch_unwind(|| (a.len(), a.iter().map(|(k,v)| (k,v.encode())).collect::<Vec<_>>()));
   println!("object {s} try={try_variant}: {result:?}");
  }
 }
 let mut b = br#"{"a":[1,2]}"#.to_vec(); let t=simd_json::to_tape(&mut b).unwrap();
 assert_eq!(t.as_value().get_array("a").unwrap().iter().count(),2);
}
#[test]
fn exhausted_numeric() {
 let mut b=b"1".to_vec(); let mut d=simd_json::Deserializer::from_slice(&mut b).unwrap();
 assert_eq!(u64::deserialize(&mut d).unwrap(),1);
 assert!(u64::deserialize(&mut d).is_err());
}
#[test]
fn tuple_boundary() {
 let got=simd_json::from_slice::<((u8,),u8)>(&mut b"[[1,2],3]".to_vec());
 println!("tuple: {got:?}"); assert!(got.is_err());
}
#[test]
fn second_root() {
 for s in ["true false","{} {}","1 2","[] []"] {
  let got=simd_json::from_slice::<serde_json::Value>(&mut s.as_bytes().to_vec());
  println!("roots {s}: {got:?}"); assert!(got.is_err());
 }
}
struct UnknownSeq;
impl Serialize for UnknownSeq {fn serialize<S:Serializer>(&self,s:S)->Result<S::Ok,S::Error>{s.serialize_seq(None)?.end()}}
struct UnknownMap;
impl Serialize for UnknownMap {fn serialize<S:Serializer>(&self,s:S)->Result<S::Ok,S::Error>{s.serialize_map(None)?.end()}}
#[test]
fn empty_unknown() {
 let seq=simd_json::to_string(&UnknownSeq).unwrap(); let pp=simd_json::to_string_pretty(&UnknownSeq).unwrap();
 let map=simd_json::to_string(&UnknownMap).unwrap(); let mpp=simd_json::to_string_pretty(&UnknownMap).unwrap();
 println!("unknown compact seq={seq:?} map={map:?}; pretty seq={pp:?} map={mpp:?}");
 assert_eq!(seq,"[]"); assert_eq!(pp,"[]"); assert_eq!(mpp,"{}");
}
#[derive(Serialize)] enum Event {Pair(u8,u8), Empty()}
#[test]
fn pretty_tuple() {
 for v in [Event::Pair(1,2),Event::Empty()] {
  let s=simd_json::to_string_pretty(&v).unwrap();println!("tuple pretty: {s:?}");
  println!("reference decode: {:?}",serde_json::from_str::<serde_json::Value>(&s));
 }
 assert!(serde_json::from_str::<serde_json::Value>(&simd_json::to_string_pretty(&Event::Pair(1,2)).unwrap()).is_ok());
}
#[test]
fn nonfinite() {
 for v in [f64::NAN,f64::INFINITY,f64::NEG_INFINITY] {
  let c=simd_json::to_string(&v); let p=simd_json::to_string_pretty(&v); let d=OwnedValue::from(v).encode();
  println!("nonfinite {v}: compact={c:?}, pretty={p:?}, DOM={d}");
 }
 assert!(matches!(simd_json::to_string(&f64::NAN), Err(_)) || simd_json::to_string(&f64::NAN).unwrap() == "null");
}
#[test]
fn exponent_wrap() {
 for s in ["1e4294967296","1e-4294967296","1e4294967297","1e-4294967297"] {
  println!("exponent {s}: {:?}",simd_json::to_owned_value(&mut s.as_bytes().to_vec()));
 }
 assert!(simd_json::to_owned_value(&mut b"1e4294967296".to_vec()).is_err());
}
#[test]
fn numeric_suffix() {
 for s in ["1234567890123456789x","[1234567890123456789x]","12345678901234567890.0x","[1.234567890123456789012345x]","[123x]"] {
  println!("suffix {s}: {:?}",simd_json::to_owned_value(&mut s.as_bytes().to_vec()));
 }
 assert!(simd_json::to_owned_value(&mut b"1234567890123456789x".to_vec()).is_err());
}
#[test]
fn incomplete_surrogate() {
 for s in [r#""\ud800""#,r#""\ud800x""#] {
  println!("incomplete {s}: {:?}",simd_json::to_owned_value(&mut s.as_bytes().to_vec()));
 }
 assert!(simd_json::to_owned_value(&mut br#""\ud800""#.to_vec()).is_err());
}
#[test]
fn surrogate_upper_bound() {
 for s in [r#""\ud800\ue000""#,r#""\udbff\uffff""#,r#""\ud800\udc00""#,r#""\udbff\udfff""#] {
  println!("surrogate {s}: {:?}",simd_json::to_owned_value(&mut s.as_bytes().to_vec()));
 }
 assert!(simd_json::to_owned_value(&mut br#""\ud800\ue000""#.to_vec()).is_err());
}
#[test]
fn consume_keys() {
 let o=owned(r#"{"42":"ok"}"#);
 println!("owned ref: {:?}",simd_json::serde::from_refowned_value::<BTreeMap<u32,String>>(&o));
 let c=simd_json::serde::from_owned_value::<BTreeMap<u32,String>>(o);println!("owned consume: {c:?}");
 let mut b=br#"{"42":"ok"}"#.to_vec();let v=simd_json::to_borrowed_value(&mut b).unwrap();
 println!("borrowed ref: {:?}",simd_json::serde::from_refborrowed_value::<BTreeMap<u32,String>>(&v));
 let bc=simd_json::serde::from_borrowed_value::<BTreeMap<u32,String>>(v);println!("borrowed consume: {bc:?}");
 assert!(c.is_ok());assert!(bc.is_ok());
}
#[test]
fn borrowed_key_fallback() {
 let escaped=serde_json::from_str::<BorrowedValue>(r#"{"a\"b":1}"#);
 let from_value=BorrowedValue::deserialize(serde_json::json!({"x":1}));
 println!("escaped: {escaped:?}, owned value: {from_value:?}");
 assert!(escaped.is_ok());assert!(from_value.is_ok());
}
#[test]
fn lazy_equality() {
 let mut b=br#"{"a":1}"#.to_vec();let t=simd_json::to_tape(&mut b).unwrap();let mut a=simd_json::lazy::Value::from_tape(t.as_value());let clone=a.clone();
 assert_eq!(a,clone);a.as_object_mut().unwrap();println!("lazy after borrow: {a:?} vs {clone:?}");assert_eq!(a,clone);
}
#[test]
fn literal_classification() {
 for s in ["falsex","truex","nullx"] {
  let e=simd_json::to_owned_value(&mut s.as_bytes().to_vec()).unwrap_err();
  println!("literal {s}: {e:?}; syntax={} data={}",e.is_syntax(),e.is_data());
 }
 let e=simd_json::to_owned_value(&mut b"falsex".to_vec()).unwrap_err();assert!(e.is_syntax());
}
#[test]
fn error_equality() {
 let e=simd_json::ErrorType::Unexpected(Some(simd_json::ValueType::String),Some(simd_json::ValueType::Bool));
 println!("error reflexivity: {}",e==e);assert_eq!(e,e);
}
#[test]
fn signed_keys() {
 let got=simd_json::from_slice::<BTreeMap<i32,u8>>(&mut br#"{"-12":7,"5":8}"#.to_vec());
 println!("signed keys {got:?}");let m=got.unwrap();assert_eq!(m.get(&-12),Some(&7));assert_eq!(m.get(&5),Some(&8));
 let json=simd_json::to_string(&m).unwrap();assert_eq!(simd_json::from_slice::<BTreeMap<i32,u8>>(&mut json.into_bytes()).unwrap(),m);
}
#[test]
fn reuse_buffers() {
 let mut buffers=simd_json::Buffers::default();
 for s in [r#"{"long":[1,2,3,4,5],"other":{"a":true}}"#,"[]", "{}", "true", "[]"] {
  let v:serde_json::Value=simd_json::serde::from_slice_with_buffers(&mut s.as_bytes().to_vec(), &mut buffers).unwrap();
  println!("reused {s}: {v}");assert_eq!(v,serde_json::from_str::<serde_json::Value>(s).unwrap());
 }
}
#[test]
fn u8_range() {
 for s in ["256","-1","255","0"] {
  let v=simd_json::from_slice::<u8>(&mut s.as_bytes().to_vec());println!("u8 {s}: {v:?}");
  assert_eq!(v.is_ok(),matches!(s,"255"|"0"));
 }
}
#[test]
fn tape_index() {
 let mut b=b"[10,[20,30],40]".to_vec();let t=simd_json::to_tape(&mut b).unwrap();let a=t.as_value().as_array().unwrap();
 let v=a.get(2).and_then(|v|v.as_u64());println!("tape index 2: {v:?}");assert_eq!(v,Some(40));
}
#[test]
#[cfg(feature="128bit")]
fn wide_accessors() {
 for (s,u,i) in [("18446744073709551616",Some(18446744073709551616u128),Some(18446744073709551616i128)),("-9223372036854775809",None,Some(-9223372036854775809i128))] {
  let mut b=s.as_bytes().to_vec();let t=simd_json::to_tape(&mut b).unwrap();let v=t.as_value();println!("wide {s}: type={:?} u128={:?} i128={:?}",v.value_type(),v.as_u128(),v.as_i128());
  println!("DOM u128={:?} i128={:?}",owned(s).as_u128(),owned(s).as_i128());
  let _ = (u,i);
 }
 let mut b=b"18446744073709551616".to_vec();let t=simd_json::to_tape(&mut b).unwrap();assert_eq!(t.as_value().as_u128(),Some(18446744073709551616));
}
#[test]
#[cfg(feature="128bit")]
fn wide_float() {
 let f=simd_json::from_slice::<f64>(&mut b"18446744073709551616".to_vec());println!("wide f64: {f:?}");assert_eq!(f.unwrap(),18446744073709551616f64);
}
