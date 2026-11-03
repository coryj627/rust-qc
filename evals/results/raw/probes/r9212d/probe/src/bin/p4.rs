use simd_json::prelude::*;
use simd_json::{OwnedValue,BorrowedValue,json};
use serde::{Serialize,Deserialize};
use std::collections::{HashMap,BTreeMap};
#[derive(Serialize,Deserialize,Debug,PartialEq,Clone)] enum E{U,N(u8),T(u8,u8),T0(),S{a:u8},S0{}}
#[derive(Serialize,Deserialize,Debug,PartialEq,Clone)] struct Unit;
#[derive(Serialize,Deserialize,Debug,PartialEq,Clone)] struct Empty{}
#[derive(Serialize,Deserialize,Debug,PartialEq,Clone)] struct NT(u8);
#[derive(Serialize,Deserialize,Debug,PartialEq,Clone)] struct TS(u8,u8);
#[derive(Serialize,Deserialize,Debug,PartialEq,Clone)] #[serde(tag="t")] enum Tg{A{x:u8},B}
#[derive(Serialize,Deserialize,Debug,PartialEq,Clone)] #[serde(untagged)] enum Ut{A(u8),B(String),C(Vec<u8>)}
#[derive(Serialize,Deserialize,Debug,PartialEq,Clone)] struct Fl{a:u8,#[serde(flatten)] r:BTreeMap<String,u8>}
#[derive(Serialize,Deserialize,Debug,PartialEq,Clone)] struct Bo{#[serde(with="bb")] b:Vec<u8>}
mod bb{pub fn serialize<S:serde::Serializer>(v:&Vec<u8>,s:S)->Result<S::Ok,S::Error>{s.serialize_bytes(v)} pub fn deserialize<'d,D:serde::Deserializer<'d>>(d:D)->Result<Vec<u8>,D::Error>{ <Vec<u8> as serde::Deserialize>::deserialize(d)}}
fn rt<T:Serialize+for<'a>Deserialize<'a>+std::fmt::Debug+PartialEq+Clone>(n:&str,t:T){
  let tv:String=match simd_json::serde::to_owned_value(t.clone()){Ok(v)=>format!("{}",v.encode()),Err(e)=>format!("ERR {e}")};
  let bv:String=match simd_json::serde::to_borrowed_value(t.clone()){Ok(v)=>format!("{}",v.encode()),Err(e)=>format!("ERR {e}")};
  let sj=serde_json::to_string(&t).unwrap_or("ERR".into());
  let mut flags=String::new(); if tv!=sj {flags+=" OWNED_DIFF";} if bv!=sj {flags+=" BORROWED_DIFF";}
  let sv=simd_json::to_string(&t).unwrap(); if sv!=sj {flags+=" STR_DIFF";}
  let mut b=sv.clone().into_bytes(); let a:Result<T,_>=simd_json::from_slice(&mut b); if a.as_ref().ok()!=Some(&t){flags+=&format!(" SLICE_RT({:?})",a.map_err(|e|e.to_string()));}
  if let Ok(v)=simd_json::serde::to_owned_value(t.clone()){ let f:Result<T,_>=simd_json::serde::from_owned_value(v.clone()); if f.as_ref().ok()!=Some(&t){flags+=&format!(" FROM_OWNED({:?})",f.map_err(|e|e.to_string()));} let f:Result<T,_>=simd_json::serde::from_refowned_value(&v); if f.as_ref().ok()!=Some(&t){flags+=&format!(" FROM_REFOWNED({:?})",f.map_err(|e|e.to_string()));} }
  if let Ok(v)=simd_json::serde::to_borrowed_value(t.clone()){ let f:Result<T,_>=simd_json::serde::from_borrowed_value(v.clone()); if f.as_ref().ok()!=Some(&t){flags+=&format!(" FROM_BORROWED({:?})",f.map_err(|e|e.to_string()));} let f:Result<T,_>=simd_json::serde::from_refborrowed_value(&v); if f.as_ref().ok()!=Some(&t){flags+=&format!(" FROM_REFBORROWED({:?})",f.map_err(|e|e.to_string()));} }
  println!("{n}: owned={tv} sj={sj}{flags}");
}
fn main(){
  rt("E::U",E::U);rt("E::N",E::N(1));rt("E::T",E::T(1,2));rt("E::T0",E::T0());rt("E::S",E::S{a:1});rt("E::S0",E::S0{});
  rt("Unit",Unit);rt("Empty",Empty{});rt("NT",NT(1));rt("TS",TS(1,2));rt("()",());rt("opt",Some(1u8));rt("none",None::<u8>);rt("optopt",Some(Some(1u8)));
  rt("Tg",Tg::A{x:1});rt("TgB",Tg::B);rt("Ut",Ut::A(1));rt("UtS",Ut::B("x".into()));rt("UtV",Ut::C(vec![1]));
  rt("Fl",Fl{a:1,r:BTreeMap::from([("z".to_string(),2)])});rt("Bo",Bo{b:vec![1,2]});
  rt("intkey",HashMap::from([(1u8,2u8)]));rt("negkey",HashMap::from([(-1i16,2u8)]));rt("strkey",HashMap::from([("a".to_string(),2u8)]));rt("charkey",HashMap::from([('c',2u8)]));rt("u64key",HashMap::from([(u64::MAX,2u8)]));rt("nestedE",vec![E::U,E::N(1)]);rt("ekey",BTreeMap::from([("a".to_string(),E::N(3))]));
  rt("u64max",u64::MAX);rt("i64min",i64::MIN);rt("f64",1.5f64);rt("f32",1.5f32);rt("f32b",0.1f32);rt("char",'x');rt("string","s\"\n".to_string());rt("tuple",(1u8,"a".to_string()));rt("arr",[1u8,2]);
  rt("u128s",5u128);rt("i128s",-5i128);
  // value api
  let mut o=json!({"a":1}); println!("json! a {:?} {}",o.get("a"),o.encode());
  println!("i64 vs u64 eq {}", json!(1)==simd_json::to_owned_value(&mut b"1".to_vec()).unwrap());
  println!("json big {}", json!(u64::MAX).encode());
  println!("json float {}", json!(1.0).encode());
  println!("json nested {}", json!({"a":[1,{"b":null}],"c":true,"d":"e"}).encode());
  println!("json expr key {}", {let k="k"; json!({k:1}).encode()});
  println!("json trailing {}", json!([1,2,]).encode());
  let bv:BorrowedValue=json!({"a":1}).into(); println!("into borrowed {}",bv.encode());
  let ov2:OwnedValue=bv.clone().into(); println!("back {}",ov2.encode());
  println!("owned==borrowed {}", ov2==bv);
  println!("borrowed==owned {}", bv==ov2);
  let st=bv.clone().into_static(); println!("into_static {}",st.encode());
  // as_* conversions
  let v=OwnedValue::from(u64::MAX); println!("u64max as_i64 {:?} as_f64 {:?} cast {:?} as_u8 {:?} usize {:?}",v.as_i64(),v.as_f64(),v.cast_f64(),v.as_u8(),v.as_usize());
  let v=OwnedValue::from(-1i64); println!("-1 as_u64 {:?} as_u8 {:?} as_i8 {:?} as_usize {:?} cast {:?}",v.as_u64(),v.as_u8(),v.as_i8(),v.as_usize(),v.cast_f64());
  let v=OwnedValue::from(1e300f64); println!("1e300 as_f32 {:?} as_f64 {:?}",v.as_f32(),v.as_f64());
  println!("eq: 1u8 {} 1i64 {} 1.0f64 {}", json!(1)==1u8, json!(1)==1i64, json!(1)==1.0f64);
  // insert on dup, remove on dup
  let mut b=br#"{"a":1,"a":2}"#.to_vec(); let mut d=simd_json::to_owned_value(&mut b).unwrap(); println!("dup insert {:?}",d.insert("a",9)); println!("after insert enc {}",d.encode()); println!("remove {:?} enc {}",d.remove("a"),d.encode());
  let mut b=br#"{"a":1,"a":2}"#.to_vec(); let d=simd_json::to_owned_value(&mut b).unwrap(); let ser=simd_json::to_string(&d).unwrap(); println!("dup serde {ser}");
  // Index panics
  // Buffers reuse with diff sizes
  let mut bufs=simd_json::Buffers::new(0); for doc in ["[1,2,3]","\"long string long string long string long string long string long string long string long string long string\\n\"","1","{\"a\":\"b\\nc\"}","[]"] { let mut b=doc.as_bytes().to_vec(); let v=simd_json::OwnedValue::from(simd_json::to_tape_with_buffers(&mut b,&mut bufs).unwrap().as_value().encode()); println!("reuse {}",v); }
  let mut b=b"[1,".to_vec(); let mut ib=b"[1,2]".to_vec(); let mut t=simd_json::to_tape(&mut ib).unwrap(); println!("fill_tape err {:?}",simd_json::fill_tape(&mut b,&mut bufs,&mut t).map_err(|e|e.to_string()));  println!("tape after error enc={:?} nodes={:?}",std::panic::catch_unwind(std::panic::AssertUnwindSafe(||t.as_value().encode())).map_err(|_|"PANIC"),t.0);
  let mut b=br#"{"a":"#.to_vec();
  let mut t2=simd_json::Tape::null(); println!("fill err {:?} {:?}",simd_json::fill_tape(&mut b,&mut bufs,&mut t2).map_err(|e|e.to_string()),t2.0);
  // mutated input on failure
  let mut b=br#"["a\nb",x]"#.to_vec(); let e=simd_json::to_owned_value(&mut b); println!("input after failed parse {:?} {:?}",String::from_utf8_lossy(&b),e.is_err());
  // from_reader
  let r:Result<Vec<u8>,_>=simd_json::from_reader(&b"[1,2]"[..]); println!("reader {r:?}");
  // deser from borrowed: zero copy, string with escape
  #[derive(Deserialize,Debug)] struct Z<'a>{#[serde(borrow)] a:&'a str, #[serde(borrow)] c:std::borrow::Cow<'a,str>}
  let mut b=br#"{"a":"x\ny","c":"plain"}"#.to_vec(); println!("zero {:?}",simd_json::from_slice::<Z>(&mut b).map_err(|e|e.to_string()));
  // serde_json conversions
  let sj:serde_json::Value=serde_json::json!({"a":[1,-1,1.5,null],"b":{"c":u64::MAX}}); let o:OwnedValue=sj.clone().try_into().unwrap(); println!("sj->owned {}",o.encode()); let back:serde_json::Value=o.try_into().unwrap(); println!("rt {}",back==sj);
  let nanv:Result<serde_json::Value,_>=OwnedValue::from(f64::NAN).try_into(); println!("nan->sj {:?}",nanv.map_err(|e:simd_json::serde::SerdeConversionError|e.to_string()));
}
