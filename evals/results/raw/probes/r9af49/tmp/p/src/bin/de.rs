use serde::{Deserialize};
use simd_json::prelude::*;
use std::collections::HashMap;
fn vec(s:&str)->Vec<u8>{s.as_bytes().to_vec()}
#[derive(Deserialize,Debug,PartialEq)] struct P{a:u8,b:u8}
#[derive(Deserialize,serde::Serialize,Debug,PartialEq)] enum E{U,N(u8),T(u8,u8),T0(),S{a:u8},}
fn main(){
  // extras
  let mut b=vec("[1,2,3]"); println!("tuple extras {:?}",simd_json::from_slice::<(u8,u8)>(&mut b));
  println!("  sj {:?}",serde_json::from_str::<(u8,u8)>("[1,2,3]").map_err(|e|e.to_string()));
  let s="[[1,2,[3,4]],[5,6]]"; let mut b=vec(s); println!("vec tuple misalign {:?}",simd_json::from_slice::<Vec<(u8,u8)>>(&mut b));
  println!("  sj {:?}",serde_json::from_str::<Vec<(u8,u8)>>(s).map_err(|e|e.to_string()));
  let s="[1,2,3]"; let mut b=vec(s); println!("struct from long array {:?}",simd_json::from_slice::<P>(&mut b));
  println!("  sj {:?}",serde_json::from_str::<P>(s).map_err(|e|e.to_string()));
  let s=r#"{"T":[1,2,[7,8]]}"#; let mut b=vec(s); println!("tuple variant extras {:?}",simd_json::from_slice::<E>(&mut b));
  println!("  sj {:?}",serde_json::from_str::<E>(s).map_err(|e|e.to_string()));
  let s=r#"[{"a":1,"b":2},{"a":3,"b":4}]"#; let mut b=vec(s); println!("[P;1]-like (P,) {:?}",simd_json::from_slice::<(P,)>(&mut b));
  // owned value
  let mut b=vec("[1,2,3]"); let v=simd_json::to_owned_value(&mut b).unwrap(); println!("from_owned tuple extras {:?}",simd_json::serde::from_owned_value::<(u8,u8)>(v));
  // int keys
  let s=r#"{"1":2}"#;
  let mut b=vec(s); println!("from_slice int key {:?}",simd_json::from_slice::<HashMap<i32,i32>>(&mut b));
  let mut b=vec(s); let v=simd_json::to_owned_value(&mut b).unwrap(); println!("from_owned int key {:?}",simd_json::serde::from_owned_value::<HashMap<i32,i32>>(v));
  let mut b=vec(s); let v=simd_json::to_owned_value(&mut b).unwrap(); println!("from_refowned int key {:?}",simd_json::serde::from_refowned_value::<HashMap<i32,i32>>(&v));
  let mut b=vec(s); let v=simd_json::to_borrowed_value(&mut b).unwrap(); println!("from_borrowed int key {:?}",simd_json::serde::from_borrowed_value::<HashMap<i32,i32>>(v));
  let mut b=vec(s); let v=simd_json::to_borrowed_value(&mut b).unwrap(); println!("from_refborrowed int key {:?}",simd_json::serde::from_refborrowed_value::<HashMap<i32,i32>>(&v));
  let v: serde_json::Value = serde_json::from_str(s).unwrap(); println!("  sj from_value int key {:?}",serde_json::from_value::<HashMap<i32,i32>>(v).map_err(|e|e.to_string()));
  // bool key from_slice
  let s=r#"{"true":2}"#; let mut b=vec(s); println!("from_slice bool key {:?}",simd_json::from_slice::<HashMap<bool,i32>>(&mut b).map_err(|e|e.to_string()));
  println!("  sj {:?}",serde_json::from_str::<HashMap<bool,i32>>(s).map_err(|e|e.to_string()));
  // i128 key
  let s=r#"{"1":2}"#; let mut b=vec(s); println!("from_slice i128 key {:?}",simd_json::from_slice::<HashMap<i128,i32>>(&mut b).map_err(|e|e.to_string()));
  println!("  sj {:?}",serde_json::from_str::<HashMap<i128,i32>>(s).map_err(|e|e.to_string()));
  println!("to_owned u128 key {:?}",{let mut h=HashMap::new();h.insert(1u128,1);simd_json::serde::to_owned_value(h).map_err(|e|e.to_string())});
  // borrowed value via non-borrowing deser
  let s=r#"{"a\nb":1}"#; println!("sj->Borrowed escaped key {:?}",serde_json::from_str::<simd_json::BorrowedValue>(s).map_err(|e|e.to_string()));
  println!("sj->Owned escaped key {:?}",serde_json::from_str::<simd_json::OwnedValue>(s).map_err(|e|e.to_string()));
  let hm:simd_json::BorrowedValue = {let mut o=simd_json::BorrowedValue::object(); o.try_insert(String::from("k"),1); o};
  println!("borrowed(owned key)->Borrowed {:?}",simd_json::serde::from_borrowed_value::<simd_json::BorrowedValue>(hm).map_err(|e|e.to_string()));
  // empty tuple variant round trip
  let v=simd_json::serde::to_owned_value(E::T0()).unwrap(); println!("T0 owned {:?} back {:?}",v,simd_json::serde::from_owned_value::<E>(v.clone()).map_err(|e|e.to_string()));
  let v2=serde_json::to_value(E::T0()).unwrap(); println!("  sj {:?}",serde_json::from_value::<E>(v2).map_err(|e|e.to_string()));
  let mut b=vec(r#"{"T0":[]}"#); println!("T0 from_slice {:?}",simd_json::from_slice::<E>(&mut b).map_err(|e|e.to_string()));
  // tape round trip
  let mut b=vec(r#"{"a":[1,2,{"b":null}],"c":"d"}"#); let t=simd_json::to_tape(&mut b).unwrap(); println!("{:?}",t);
  // dup keys
  let s=r#"{"a":1,"a":2}"#; let mut b=vec(s); let v=simd_json::to_owned_value(&mut b).unwrap(); println!("dup owned {:?} a={:?} len={:?} encode={}",v,v.get("a"),v.as_object().map(|o|o.len()),v.encode());
  let mut big=String::from("{"); for i in 0..200 { big+=&format!("\"k{}\":{},",i%50,i);} big+="\"z\":0}";
  let mut b=vec(&big); let v=simd_json::to_owned_value(&mut b).unwrap(); println!("dup big len={:?} k0={:?}",v.as_object().map(|o|o.len()),v.get("k0"));
  let mut o=v.clone(); let r=o.try_insert("k0",999); println!("insert over dup: {:?} get={:?} ",r,o.get("k0"));
  let r=o.remove("k0"); println!("remove {:?} then get {:?}",r,o.get("k0"));
  let sj:serde_json::Value=serde_json::from_str(&big).unwrap(); println!("sj k0 {:?} len {}",sj.get("k0"),sj.as_object().unwrap().len());
  let mut b=vec(s); println!("from_slice HashMap dup {:?}",simd_json::from_slice::<HashMap<String,u8>>(&mut b));
  let mut b=vec(s); println!("from_slice struct dup {:?}",simd_json::from_slice::<PA>(&mut b).map_err(|e|e.to_string()));
  // error eq
  println!("err eq InputTooLarge {:?}",simd_json::ErrorType::InputTooLarge==simd_json::ErrorType::InputTooLarge);
  println!("err eq SimdUnsupported {:?}",simd_json::ErrorType::SimdUnsupported==simd_json::ErrorType::SimdUnsupported);
  println!("err eq Unexpected {:?}",simd_json::ErrorType::Unexpected(None,None)==simd_json::ErrorType::Unexpected(None,None));
  println!("err eq Io {:?}",simd_json::ErrorType::Io(std::io::Error::other("a"))==simd_json::ErrorType::Io(std::io::Error::other("b")));
  // OOB
  let a=std::env::args().nth(1).unwrap_or_default();
  if a=="oob" {
    let mut b=vec("1"); let mut d=simd_json::Deserializer::from_slice(&mut b).unwrap();
    println!("first {:?}",u8::deserialize(&mut d));
    println!("second {:?}",u8::deserialize(&mut d));
  }
  if a=="tapeoob" { let t=simd_json::tape::Tape(vec![]); println!("{:?}",t.deserialize::<u8>()); }
  if a=="deep" { let n:usize=std::env::args().nth(2).unwrap().parse().unwrap(); let s=format!("{}{}","[".repeat(n),"]".repeat(n)); let mut b=vec(&s);
     let which=std::env::args().nth(3).unwrap();
     match which.as_str(){
      "owned"=>{let v=simd_json::to_owned_value(&mut b).unwrap(); println!("ok {}",v.is_array()); std::mem::forget(v)},
      "borrowed"=>{let v=simd_json::to_borrowed_value(&mut b).unwrap(); println!("ok {}",v.is_array());std::mem::forget(v)},
      "tape"=>{let v=simd_json::to_tape(&mut b).unwrap(); println!("tape ok {}",v.0.len())},
      "serde"=>{let v=simd_json::from_slice::<simd_json::OwnedValue>(&mut b); println!("{}",v.is_ok());std::mem::forget(v)},
      "ignored"=>{let v=simd_json::from_slice::<serde::de::IgnoredAny>(&mut b); println!("{}",v.is_ok())},
      _=>{}
     }
  }
}
#[derive(Deserialize,Debug)] struct PA{#[allow(dead_code)]a:u8}
