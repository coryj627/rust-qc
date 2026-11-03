use simd_json::prelude::*;
use simd_json::{BorrowedValue,OwnedValue};
use serde::{Serialize,Deserialize};
use std::collections::{HashMap,BTreeMap};
fn try_panic<T:std::fmt::Debug>(name:&str,f:impl FnOnce()->T+std::panic::UnwindSafe){ match std::panic::catch_unwind(f){Ok(v)=>println!("{name}: {v:?}"),Err(e)=>println!("{name}: PANIC {:?}",e.downcast_ref::<String>().map(|s|s.as_str()).or(e.downcast_ref::<&str>().copied()))} }
#[derive(Serialize,Deserialize,Debug,PartialEq,PartialOrd,Ord,Eq)] enum E{U,N(u8),T(u8,u8),T0(),S{a:u8},S0{}}
#[derive(Serialize,Deserialize,Debug,PartialEq)] struct Unit;
#[derive(Serialize,Deserialize,Debug,PartialEq)] struct Empty{}
#[derive(Serialize,Deserialize,Debug,PartialEq)] struct One{a:u8}
#[derive(Serialize,Deserialize,Debug,PartialEq)] struct Skip{#[serde(skip_serializing_if="Option::is_none")] a:Option<u8>}
#[derive(Serialize,Deserialize,Debug,PartialEq)] struct TS(u8,u8);
#[derive(Serialize,Deserialize,Debug,PartialEq)] struct TS0();
#[derive(Serialize,Deserialize,Debug,PartialEq)] struct Pair{a:(u8,),b:u8}
fn ser<T:Serialize+?Sized>(n:&str,t:&T){ let a=simd_json::to_string(t).map_err(|e|e.to_string()); let b=serde_json::to_string(t).map_err(|e|e.to_string());let p=simd_json::to_string_pretty(t).map_err(|e|e.to_string());let q=serde_json::to_string_pretty(t).map_err(|e|e.to_string()); println!("ser {n}: simd={a:?} serde_json={b:?}{}", if a==b {""} else {" DIFF"}); if p!=q {println!("   pretty DIFF simd={p:?} serde_json={q:?}");} }
struct Unsized_(Vec<u8>); impl Serialize for Unsized_{fn serialize<S:serde::Serializer>(&self,s:S)->Result<S::Ok,S::Error>{ s.collect_seq(self.0.iter().filter(|_|true)) }}
struct UM(Vec<(u8,u8)>); impl Serialize for UM{fn serialize<S:serde::Serializer>(&self,s:S)->Result<S::Ok,S::Error>{ s.collect_map(self.0.iter().filter(|_|true).map(|(a,b)|(a,b))) }}
fn de<T:for<'a>Deserialize<'a>+std::fmt::Debug>(n:&str,s:&str){ let mut b=s.as_bytes().to_vec(); let a=simd_json::from_slice::<T>(&mut b).map_err(|e|e.to_string()); let c=serde_json::from_str::<T>(s).map_err(|e|e.to_string()); println!("de {n} {s}: simd={a:?} serde_json={c:?}"); }
fn main(){
  // tape get_array etc
  let mut i=br#"{"k":[1,2,3],"o":{"a":1,"b":2},"z":9}"#.to_vec();
  let t=simd_json::to_tape(&mut i).unwrap(); let v=t.as_value();
  println!("via get->as_array: {:?}", v.get("k").unwrap().as_array().unwrap().iter().map(|x|x.as_u64()).collect::<Vec<_>>());
  let a=v.get_array("k").unwrap(); println!("get_array len={} iter={:?} get2={:?}", a.len(), a.iter().map(|x|x.as_u64()).collect::<Vec<_>>(), a.get(2).map(|x|x.as_u64()));
  let a=v.try_get_array("k").unwrap().unwrap(); println!("try_get_array iter={:?}", a.iter().map(|x|x.as_u64()).collect::<Vec<_>>());
  let o=v.get_object("o").unwrap(); println!("get_object len={} a={:?} b={:?} iter={:?}", o.len(), o.get("a").map(|x|x.as_u64()), o.get("b").map(|x|x.as_u64()), o.iter().map(|(k,x)|(k,x.as_u64())).collect::<Vec<_>>());
  let o2=v.try_get_object("o").unwrap().unwrap(); println!("try_get_object b={:?}", o2.get("b").map(|x|x.as_u64()));
  // empties
  let mut i=br#"{"k":[],"o":{}}"#.to_vec(); let t=simd_json::to_tape(&mut i).unwrap(); let v=t.as_value();
  println!("empty get_array {:?}", std::panic::catch_unwind(||0).is_ok());
  let r=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||{ let a=v.get_array("k"); a.map(|a|a.len()) })); println!("empty get_array len: {:?}",r.map_err(|_|"PANIC"));
  let r=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||{ let a=v.get_object("o"); a.map(|a|a.len()) })); println!("empty get_object len: {:?}",r.map_err(|_|"PANIC"));
  // lazy
  let mut i=br#"{"k":[1,2,3],"o":{"a":1}}"#.to_vec(); let t=simd_json::to_tape(&mut i).unwrap(); let tv=t.as_value();
  let l=simd_json::value::lazy::Value::from_tape(tv); println!("lazy get k? {:?}", l.get("k").map(|x|x.is_array()));
  println!("lazy get_idx {:?}", simd_json::value::lazy::Value::from_tape(tv).get("k").and_then(|x|x.get_idx(2).map(|y|y.as_u64())));
  // tape Value on tape::null, tape reset then as_value
  let t2=simd_json::Tape::null().reset(); let tv2=t2.as_value();
  try_panic("empty tape value is_null",std::panic::AssertUnwindSafe(||tv2.as_null()));
  try_panic("empty tape value_type",std::panic::AssertUnwindSafe(||tv2.value_type()));
  try_panic("empty tape encode",std::panic::AssertUnwindSafe(||tv2.encode()));
  // bogus tape via pub field
  let bogus=simd_json::Tape(vec![simd_json::Node::Array{len:1,count:5}]); 
  try_panic("bogus tape as_array",std::panic::AssertUnwindSafe(||bogus.as_value().as_array().map(|a|a.len())));
  // dup keys
  let mut small=br#"{"a":1,"a":2}"#.to_vec(); let ov=simd_json::to_owned_value(&mut small).unwrap(); println!("dup owned get a={:?} len={} enc={}",ov.get("a"),ov.as_object().unwrap().len(),ov.encode());
  let mut small=br#"{"a":1,"a":2}"#.to_vec(); let t=simd_json::to_tape(&mut small).unwrap(); println!("dup tape get a={:?}",t.as_value().get("a").and_then(|v|v.as_u64()));
  let mut s=String::from("{"); for i in 0..40 { s+=&format!("\"k{i}\":{i},"); } s+="\"k1\":100,\"k39\":200}"; let mut b=s.clone().into_bytes(); let ov=simd_json::to_owned_value(&mut b).unwrap(); println!("dup large owned k1={:?} k39={:?} len={}",ov.get("k1"),ov.get("k39"),ov.as_object().unwrap().len());
  let mut b=s.clone().into_bytes(); let m:HashMap<String,u32>=simd_json::from_slice(&mut b).unwrap(); println!("dup via serde HashMap k1={:?} len={}",m.get("k1"),m.len());
  let mut small=br#"{"a":1,"a":2}"#.to_vec(); let m:One=simd_json::from_slice(&mut small).map_err(|e|println!("dup field: {e}")).unwrap_or(One{a:0}); println!("dup struct {m:?}; serde_json {:?}", serde_json::from_str::<One>(r#"{"a":1,"a":2}"#).map_err(|e|e.to_string()));
  let mut ob=br#"{"a":1,"a":2}"#.to_vec(); let ov=simd_json::to_owned_value(&mut ob).unwrap(); let mut ob2=br#"{"a":2,"a":1}"#.to_vec(); let ov2=simd_json::to_owned_value(&mut ob2).unwrap(); println!("dup eq {}",ov==ov2);
  // serialize
  ser("unit",&());ser("Unit",&Unit);ser("Empty",&Empty{});ser("One",&One{a:1});ser("Skip none",&Skip{a:None});ser("TS",&TS(1,2));ser("TS0",&TS0());
  ser("E::U",&E::U);ser("E::N",&E::N(1));ser("E::T",&E::T(1,2));ser("E::T0",&E::T0());ser("E::S",&E::S{a:1});ser("E::S0",&E::S0{});
  ser("vec empty",&Vec::<u8>::new());ser("vec",&vec![1,2]);ser("map empty",&HashMap::<String,u8>::new());ser("tuple",&(1,2));ser("unit tuple arr",&[0u8;0]);
  ser("unsized empty",&Unsized_(vec![]));ser("unsized",&Unsized_(vec![1]));ser("umap empty",&UM(vec![]));ser("umap",&UM(vec![(1,2)]));
  ser("nan",&f64::NAN);ser("inf",&f64::INFINITY);ser("nan32",&f32::NAN);ser("f32",&0.1f32);ser("f64",&0.1f64);ser("1.0",&1.0f64);ser("1e300",&1e300f64);ser("-0",&-0.0f64);
  ser("char q",&'"');ser("char nl",&'\n');ser("str esc","a\"b\\c\n\t\u{1}\u{7f}é😀");ser("bytes",&serde_bytes_like());ser("i128",&i128::MAX);ser("u128",&u128::MAX);ser("opt",&Some(1));ser("none",&None::<u8>);
  let mut mk=BTreeMap::new(); mk.insert(1u8,1u8); ser("int key",&mk);
  let mut mk=BTreeMap::new(); mk.insert(-1i64,1u8); ser("neg key",&mk);
  let mut mk=BTreeMap::new(); mk.insert(true,1u8); ser("bool key",&mk);
  let mut mk=BTreeMap::new(); mk.insert('"',1u8); ser("char key",&mk);
  let mut mk=BTreeMap::new(); mk.insert("a\"b".to_string(),1u8); ser("esc key",&mk);
  let mut mk=BTreeMap::new(); mk.insert(E::U,1u8); ser_err(&mk);
  #[derive(Serialize)] struct Q{#[serde(rename="a\"b")] x:u8} ser("esc field",&Q{x:1});
  #[derive(Serialize)] enum QE{#[serde(rename="a\"b")] V,#[serde(rename="c\"d")] W(u8)} ser("esc variant",&QE::V); ser("esc nvariant",&QE::W(1));
  // deserialize
  de::<(u8,u8)>("tuple long","[1,2,3]"); de::<(u8,u8)>("tuple short","[1]"); de::<[u8;2]>("arr long","[1,2,3]"); de::<TS>("TS long","[1,2,3]"); de::<Vec<(u8,)>>("misalign","[[1,[2]],[9]]");
  de::<Pair>("pair","{\"a\":[1,2],\"b\":3}"); de::<One>("struct seq long","[1,2]"); de::<Vec<One>>("struct seq","[[1,2],[3]]");
  de::<E>("enum","\"U\""); de::<E>("enum obj","{\"N\":1}"); de::<E>("enum U obj","{\"U\":null}"); de::<E>("enum T long","{\"T\":[1,2,3]}"); de::<E>("enum S","{\"S\":{\"a\":1}}"); de::<E>("enum 2 keys","{\"N\":1,\"U\":null}"); de::<Vec<E>>("enum then","[{\"T\":[1,2,3]},\"U\"]");
  de::<f64>("float int","1"); de::<f32>("f32 big","1e300"); de::<u8>("u8 float","1.0"); de::<u8>("u8 over","300"); de::<i8>("i8 neg","-1"); de::<u8>("neg","-1"); de::<f64>("f64 null","null"); de::<Option<u8>>("opt","null"); de::<()>("unit","null"); de::<char>("char","\"a\""); de::<char>("char2","\"ab\""); de::<String>("str","\"a\\nb\""); de::<Vec<u8>>("bytes","[1,2]"); de::<bool>("bool","1");
  de::<HashMap<u8,u8>>("intkey","{\"1\":2}"); de::<HashMap<bool,u8>>("boolkey","{\"true\":2}"); de::<HashMap<String,u8>>("noterm","{\"a\":1}");  de::<BTreeMap<u8,u8>>("badintkey","{\"x\":2}");
  de::<serde_json::Value>("any","{\"a\":[1,-2,3.5,null,true,\"x\"]}");
  de::<u128>("u128","340282366920938463463374607431768211455"); de::<u128>("u128s","5"); de::<i128>("i128s","-5"); de::<HashMap<i128,u8>>("i128key","{\"5\":1}"); de::<HashMap<u128,u8>>("u128key","{\"5\":1}");
  de::<OwnedValue>("ov","{\"a\":[1,-2,3.5,null,true,\"x\"]}");
  let mut b=br#"{"a":"x\ny"}"#.to_vec(); println!("bv {:?}",simd_json::to_borrowed_value(&mut b).map(|v:BorrowedValue|v.encode()));
  // error index
  for s in ["[1,2,\"\\x\"]","[1,2,3,4,5,6,\"abc\\uZZZZ\"]","[1,2,3,4,\"abc\\uDC00\"]","[1,2,  1.5x]","[    \"\u{1}\"]"] { let mut b=s.as_bytes().to_vec(); let e=simd_json::to_owned_value(&mut b).unwrap_err(); println!("errpos {s:?}: idx={} ch={:?} {:?}",e.index(),e.character(),e.error()); }
  // value encoding
  let n=OwnedValue::from(f64::NAN); println!("encode NaN {}",n.encode()); println!("encode inf {}",OwnedValue::from(f64::INFINITY).encode()); println!("pretty NaN {}",n.encode_pp());
  println!("display string {}", OwnedValue::from("a\"b"));
  println!("display obj {}", simd_json::json!({"a":"x\"y","b":[1,"q"]}));
  println!("enc esc {}", OwnedValue::from("a\"b\\\n\u{1}é").encode());
  println!("error eq {} {}", simd_json::ErrorType::InputTooLarge==simd_json::ErrorType::InputTooLarge, simd_json::ErrorType::SimdUnsupported==simd_json::ErrorType::SimdUnsupported);
}
fn serde_bytes_like()->Bytes{Bytes(vec![1,2])}
struct Bytes(Vec<u8>); impl Serialize for Bytes{fn serialize<S:serde::Serializer>(&self,s:S)->Result<S::Ok,S::Error>{s.serialize_bytes(&self.0)}}
fn ser_err<T:Serialize>(t:&T){ println!("enum-key: {:?} / {:?}", simd_json::to_string(t).map_err(|e|e.to_string()), serde_json::to_string(t).map_err(|e|e.to_string())); }
