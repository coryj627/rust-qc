use serde::{Serialize, Serializer, Deserialize};
use serde::ser::{SerializeSeq, SerializeMap, SerializeStruct, SerializeTupleVariant};
use simd_json::prelude::*;
use std::collections::HashMap;
struct EmptyNoneSeq;
impl Serialize for EmptyNoneSeq { fn serialize<S:Serializer>(&self,s:S)->Result<S::Ok,S::Error>{ let q=s.serialize_seq(None)?; q.end() } }
struct EmptyNoneMap;
impl Serialize for EmptyNoneMap { fn serialize<S:Serializer>(&self,s:S)->Result<S::Ok,S::Error>{ let q=s.serialize_map(None)?; q.end() } }
struct EmptyStruct2;
impl Serialize for EmptyStruct2 { fn serialize<S:Serializer>(&self,s:S)->Result<S::Ok,S::Error>{ let q=s.serialize_struct("x",2)?; q.end() } }
#[derive(Serialize)] enum E { U, N(i32), T(i32,i32), T0(), S{a:i32,b:i32}, S0{}, #[serde(rename="we\"ird")] W, #[serde(rename="ta\"b")] W2(i32) }
#[derive(Serialize)] struct St { a:i32, b:i32, #[serde(rename="q\"k")] c:i32 }
#[derive(Serialize)] struct Fl { #[serde(flatten)] m: HashMap<String,i32> }
#[derive(Serialize)] struct Ne { x: Vec<i32>, y: St, z: E, w: Vec<i32> }
fn sh<T:Serialize>(n:&str,t:&T){
  let c = simd_json::to_string(t);
  let p = simd_json::to_string_pretty(t);
  let sj = serde_json::to_string(t);
  let sjp = serde_json::to_string_pretty(t);
  println!("--- {n}");
  match &c { Ok(s)=>println!("compact {:?} valid={}",s,serde_json::from_str::<serde_json::Value>(s).is_ok()), Err(e)=>println!("compact err {e:?}") }
  match &p { Ok(s)=>println!("pretty {:?} valid={}",s,serde_json::from_str::<serde_json::Value>(s).is_ok()), Err(e)=>println!("pretty err {e:?}") }
  println!("sj {:?}\nsjp {:?}",sj.map_err(|e|e.to_string()),sjp.map_err(|e|e.to_string()));
}
fn main(){
  sh("seq none empty",&EmptyNoneSeq);
  sh("vec of seq none empty",&vec![EmptyNoneSeq,EmptyNoneSeq]);
  sh("map none empty",&EmptyNoneMap);
  sh("struct len2 no fields",&EmptyStruct2);
  sh("unit",&E::U); sh("newtype",&E::N(1)); sh("tuple",&E::T(1,2)); sh("tuple0",&E::T0());
  sh("struct var",&E::S{a:1,b:2}); sh("struct0",&E::S0{}); sh("W",&E::W); sh("W2",&E::W2(1));
  sh("st",&St{a:1,b:2,c:3});
  sh("flatten empty",&Fl{m:HashMap::new()});
  sh("nested",&Ne{x:vec![],y:St{a:1,b:2,c:3},z:E::T(1,2),w:vec![1]});
  sh("nested2",&vec![E::S{a:1,b:2},E::S0{},E::T(1,2),E::N(1)]);
  sh("f32",&0.1f32); sh("f64nan",&f64::NAN); sh("inf",&f64::INFINITY); sh("neginf",&f64::NEG_INFINITY); sh("f32nan",&f32::NAN);
  sh("char q",&'"'); sh("char bs",&'\\'); sh("ctrl",&'\n'); sh("str",&"a\"b\\c\n\u{1}\u{7f}é");
  sh("bytes",&serde_bytes_like(&[1,2,3]));
  let mut hm=HashMap::new(); hm.insert('"',1); sh("charkey",&hm);
  let mut hm=HashMap::new(); hm.insert(1i32,1); sh("intkey",&hm);
  let mut hm=HashMap::new(); hm.insert(true,1); sh("boolkey",&hm);
  sh("i128",&i128::MIN); sh("u128",&u128::MAX);
  // Value encode
  let v = simd_json::OwnedValue::from(f64::NAN); println!("owned nan encode {:?}",v.encode());
  let v = simd_json::OwnedValue::from(f64::INFINITY); println!("owned inf encode {:?}",v.encode());
  let v = simd_json::BorrowedValue::from(f64::NAN); println!("borrowed nan encode {:?}",v.encode());
  let mut o = simd_json::OwnedValue::object(); o.try_insert("a\"b\n", 1); println!("{}",o.encode()); println!("{}",o.encode_pp());
  // serde to_value NaN
  println!("to_value nan {:?}",simd_json::serde::to_owned_value(f64::NAN));
  println!("to_value f32 {:?}",simd_json::serde::to_owned_value(0.1f32));
  println!("sj to_value f32 {:?}",serde_json::to_value(0.1f32));
  let _ : Option<E> = None;
  fn _t<'a,T:Deserialize<'a>>(){}
}
struct B<'a>(&'a [u8]);
impl Serialize for B<'_> { fn serialize<S:Serializer>(&self,s:S)->Result<S::Ok,S::Error>{ s.serialize_bytes(self.0) } }
fn serde_bytes_like(b:&[u8])->B<'_>{B(b)}
