use simd_json::prelude::*;
use serde::{Serialize, Deserialize, Serializer};
use std::collections::{HashMap, BTreeMap};
fn deep(n: usize) -> Vec<u8> { let mut v = vec![b'['; n]; v.extend(vec![b']'; n]); v }
fn deepo(n: usize) -> Vec<u8> { let mut v = Vec::new(); for _ in 0..n { v.extend_from_slice(b"{\"a\":"); } v.push(b'1'); v.extend(vec![b'}'; n]); v }
struct FilterSeq; impl Serialize for FilterSeq { fn serialize<S:Serializer>(&self, s:S)->Result<S::Ok,S::Error>{ s.collect_seq([1,2,3].iter().filter(|x| **x>5)) } }
struct FilterMap; impl Serialize for FilterMap { fn serialize<S:Serializer>(&self, s:S)->Result<S::Ok,S::Error>{ s.collect_map([(1,2)].iter().filter(|x| x.0>5).map(|x|(x.0,x.1))) } }
struct SeqNone; impl Serialize for SeqNone { fn serialize<S:Serializer>(&self, s:S)->Result<S::Ok,S::Error>{ use serde::ser::SerializeSeq; let mut q=s.serialize_seq(None)?; q.serialize_element(&1)?; q.end() } }
#[derive(Serialize,Deserialize,Debug,PartialEq,Clone)] enum E { U, #[serde(rename="q\"x")] Q, N(u8), T(u8,u8), T0(), S{a:u8}, S0{}, #[serde(rename="st\"r")] R{ #[serde(rename="k\"ey")] a:u8}}
#[derive(Serialize,Deserialize,Debug,PartialEq,Clone)] struct Empty{}
#[derive(Serialize,Deserialize,Debug,PartialEq,Clone)] struct U1; 
#[derive(Serialize,Deserialize,Debug,PartialEq,Clone)] struct Skip{ #[serde(skip_serializing_if="Option::is_none")] a:Option<u8>, #[serde(skip_serializing_if="Option::is_none")] b:Option<u8>}
#[derive(Serialize,Deserialize,Debug,PartialEq,Clone)] struct Fl{ x:u8, #[serde(flatten)] m:HashMap<String,u8>}
#[derive(Serialize,Deserialize,Debug,PartialEq,Clone)] #[serde(tag="t")] enum Tag{A{a:u8},B}
#[derive(Serialize,Deserialize,Debug,PartialEq,Clone)] #[serde(untagged)] enum Unt{I(i64),F(f64),S(String),V(Vec<u8>)}
fn cmp<T: Serialize>(name:&str, t:&T){ let a=simd_json::to_string(t).map_err(|e|e.to_string()); let b=serde_json::to_string(t).map_err(|e|e.to_string()); let pa=simd_json::to_string_pretty(t).map_err(|e|e.to_string()); let pb=serde_json::to_string_pretty(t).map_err(|e|e.to_string());
 let v=simd_json::serde::to_owned_value(t).map(|v|simd_json::to_string(&v).unwrap()).map_err(|e|e.to_string()); println!("{name}: {} simd={a:?} sj={b:?}", if a==b {"SAME"} else {"DIFF"}); if pa!=pb { println!("   pretty simd={pa:?} sj={pb:?}"); } if v!=b {println!("   to_value={v:?}");}}
fn main(){
 let args:Vec<String>=std::env::args().collect();
 let mode=args.get(1).map(|s|s.as_str()).unwrap_or("");
 match mode {
  "owned"=>{ let n:usize=args[2].parse().unwrap(); let mut b=deep(n); let v=simd_json::to_owned_value(&mut b).unwrap(); std::mem::forget(v); println!("ok"); }
  "borrowed"=>{ let n:usize=args[2].parse().unwrap(); let mut b=deep(n); let v=simd_json::to_borrowed_value(&mut b).unwrap(); std::mem::forget(v); println!("ok"); }
  "tape"=>{ let n:usize=args[2].parse().unwrap(); let mut b=deep(n); let v=simd_json::to_tape(&mut b).unwrap(); println!("ok {}", v.0.len()); println!("enc {}", v.as_value().encode().len()); }
  "serde"=>{ let n:usize=args[2].parse().unwrap(); let mut b=deep(n); let v:Result<simd_json::OwnedValue,_>=simd_json::from_slice(&mut b); println!("ok {}", v.is_ok()); std::mem::forget(v);}
  "sj"=>{ let n:usize=args[2].parse().unwrap(); let b=deep(n); let v:Result<serde_json::Value,_>=serde_json::from_slice(&b); println!("serde_json {:?}", v.map(|_|()).map_err(|e|e.to_string())); }
  "obj"=>{ let n:usize=args[2].parse().unwrap(); let mut b=deepo(n); let v=simd_json::to_owned_value(&mut b).unwrap(); std::mem::forget(v); println!("ok"); }
  "ub1"=>{ let t=simd_json::Tape::null().reset(); println!("{:?}", t.deserialize::<u64>().map_err(|e|e.to_string())); }
  "ub2"=>{ use serde::Deserialize; let mut b=b"[1]".to_vec(); let mut d=simd_json::Deserializer::from_slice(&mut b).unwrap(); let a=<Vec<u64>>::deserialize(&mut d); let mut out=vec![]; for _ in 0..8 { out.push(u64::deserialize(&mut d).map_err(|e|e.to_string())); } println!("{a:?} {out:?}"); }
  "ub3"=>{ let mut b=b"[".to_vec(); let mut buf=simd_json::Buffers::default(); let mut b2=b"".to_vec(); let _=&mut b; let mut t=simd_json::Tape::null(); let r=simd_json::fill_tape(&mut b2,&mut buf,&mut t); println!("{:?} len {}", r.map_err(|e|e.to_string()), t.0.len()); let l=simd_json::value::lazy::Value::from_tape(t.as_value()); println!("{:?}", l.into_value()); }
  _=>{
   // tape get_array
   let mut b=br#"{"a":[1,2,3],"b":{"x":1,"y":2},"c":[1,[2,3]],"d":{"p":{"q":1}}}"#.to_vec(); let t=simd_json::to_tape(&mut b).unwrap(); let v=t.as_value();
   let a=v.get_array("a").unwrap(); println!("get_array len={} iter={:?} get2={:?}", a.len(), a.iter().map(|x|x.as_u64()).collect::<Vec<_>>(), a.get(2).map(|x|x.as_u64()));
   let aok=v.get("a").unwrap().as_array().unwrap(); println!("as_array iter={:?}", aok.iter().map(|x|x.as_u64()).collect::<Vec<_>>());
   let o=v.get_object("b").unwrap(); println!("get_object len={} get y={:?} keys={:?}", o.len(), o.get("y").map(|x|x.as_u64()), o.keys().collect::<Vec<_>>());
   let r=std::panic::catch_unwind(||{ let mut b=br#"{"c":[1,[2,3]]}"#.to_vec(); let t=simd_json::to_tape(&mut b).unwrap(); let v=t.as_value(); let c=v.get_array("c").unwrap(); c.iter().count()}); println!("nested get_array iter {:?}", r.map_err(|_|"panic"));
   let r=std::panic::catch_unwind(||{ let mut b=br#"{"d":{"p":{"q":1}}}"#.to_vec(); let t=simd_json::to_tape(&mut b).unwrap(); let v=t.as_value(); let d=v.get_object("d").unwrap(); d.iter().count()}); println!("nested get_object iter {:?}", r.map_err(|_|"panic"));
   let r=std::panic::catch_unwind(||{ let mut b=br#"{"a":[],"d":{}}"#.to_vec(); let t=simd_json::to_tape(&mut b).unwrap(); let v=t.as_value(); (v.get_array("a").map(|a|a.len()), v.try_get_object("d").map(|o|o.map(|o|o.len())).is_ok()) }); println!("empty get_array {:?}", r.map_err(|_|"panic"));
   // serializer
   cmp("filterseq",&FilterSeq); cmp("filtermap",&FilterMap); cmp("seqnone",&SeqNone); cmp("emptyvec",&Vec::<u8>::new()); cmp("emptymap",&HashMap::<String,u8>::new());
   for c in ['"','\\','\n','\u{1}','é','😀'] { cmp(&format!("char {c:?}"),&c); }
   for e in [E::U,E::Q,E::N(1),E::T(1,2),E::T0(),E::S{a:1},E::S0{},E::R{a:1}] { cmp(&format!("{e:?}"),&e); }
   cmp("empty",&Empty{}); cmp("unit",&U1); cmp("skip0",&Skip{a:None,b:None}); cmp("skip1",&Skip{a:Some(1),b:None}); cmp("skip2",&Skip{a:None,b:Some(1)});
   cmp("fl",&Fl{x:1,m:HashMap::new()}); cmp("tag",&Tag::A{a:1}); cmp("tagb",&Tag::B); cmp("unt",&vec![Unt::I(1),Unt::F(1.5),Unt::S("a".into()),Unt::V(vec![1])]);
   cmp("opt",&Some(Some(1))); cmp("none",&Option::<u8>::None); cmp("tuple0",&()); cmp("t",&(1,"a",2.5)); cmp("str",&"a\"b\\c\n\t\r\u{8}\u{c}\u{1f}\u{7f}é"); cmp("bytes",&serde_bytes_like()); cmp("f",&vec![1.0f64,0.0,-0.0,1e300,1e-300,f64::MAX,f64::MIN_POSITIVE,5e-324]); cmp("f32s",&vec![0.1f32,1.5,f32::MAX]); cmp("nanv",&vec![f64::NAN,f64::INFINITY,f64::NEG_INFINITY]);
   let mut bm=BTreeMap::new(); bm.insert("k\"ey\n",1); cmp("btree",&bm);
   // values with escaped keys
   for src in [r#"{"k\"ey\n\\":"v\"\n"}"#, r#"{"é😀":["a\/b"]}"#] { let mut b=src.as_bytes().to_vec(); let v=simd_json::to_owned_value(&mut b).unwrap(); let e=v.encode(); let p=v.encode_pp(); println!("enc {e} | pp {p:?} | ser {:?}", simd_json::to_string(&v).map_err(|e|e.to_string())); let mut b=src.as_bytes().to_vec(); let t=simd_json::to_tape(&mut b).unwrap(); println!(" tape {} | {:?}", t.as_value().encode(), t.as_value().encode_pp()); let mut bb=e.clone().into_bytes(); let vv=simd_json::to_owned_value(&mut bb).unwrap(); println!(" rt eq {}", v==vv); }
   let v=simd_json::OwnedValue::from(f64::NAN); println!("nan encode {} to_string {:?}", v.encode(), simd_json::to_string(&v).map_err(|e|e.to_string())); let mut bb=v.encode().into_bytes(); println!(" reparse {:?}", simd_json::to_owned_value(&mut bb).map_err(|e|e.to_string()));
   // from_value
   let mut m=HashMap::new(); m.insert(1u32,2u32); let ov=simd_json::serde::to_owned_value(&m).unwrap(); println!("ov {:?}", ov);
   println!("from_value intkey {:?}", simd_json::serde::from_owned_value::<HashMap<u32,u32>>(ov.clone()).map_err(|e|e.to_string()));
   println!("from_refvalue intkey {:?}", simd_json::serde::from_refowned_value::<HashMap<u32,u32>>(&ov).map_err(|e|e.to_string()));
   let bv=simd_json::serde::to_borrowed_value(&m).unwrap(); println!("from_bv intkey {:?}", simd_json::serde::from_borrowed_value::<HashMap<u32,u32>>(bv.clone()).map_err(|e|e.to_string())); println!("from_refbv intkey {:?}", simd_json::serde::from_refborrowed_value::<HashMap<u32,u32>>(&bv).map_err(|e|e.to_string()));
   for e in [E::U,E::N(1),E::T(1,2),E::T0(),E::S{a:1},E::S0{}] { let ov=simd_json::serde::to_owned_value(&e).unwrap(); let bv=simd_json::serde::to_borrowed_value(&e).unwrap(); let s=simd_json::to_string(&e).unwrap(); let mut sb=s.clone().into_bytes(); println!("rt {e:?}: owned {:?} ref {:?} borrowed {:?} refb {:?} slice {:?}", simd_json::serde::from_owned_value::<E>(ov.clone()).map_err(|e|e.to_string()), simd_json::serde::from_refowned_value::<E>(&ov).map_err(|e|e.to_string()), simd_json::serde::from_borrowed_value::<E>(bv.clone()).map_err(|e|e.to_string()), simd_json::serde::from_refborrowed_value::<E>(&bv).map_err(|e|e.to_string()), simd_json::from_slice::<E>(&mut sb).map_err(|e|e.to_string())); }
   let t=Tag::A{a:1}; let s=simd_json::to_string(&t).unwrap(); let mut b=s.into_bytes(); println!("tag {:?}", simd_json::from_slice::<Tag>(&mut b).map_err(|e|e.to_string()));
   let mut b=br#"[1,1.5,"a",[1]]"#.to_vec(); println!("untagged {:?}", simd_json::from_slice::<Vec<Unt>>(&mut b).map_err(|e|e.to_string()));
   let mut b=br#"{"x":1,"y":2}"#.to_vec(); println!("flatten {:?}", simd_json::from_slice::<Fl>(&mut b).map_err(|e|e.to_string()));
   let mut b=br#"{"a":"\n"}"#.to_vec(); #[derive(Deserialize,Debug)] struct Bs<'a>{ a:&'a str} println!("borrow escaped {:?}", simd_json::from_slice::<Bs>(&mut b).map_err(|e|e.to_string()));
   let mut b=br#"{"S":{"a":1},"x":2}"#.to_vec(); println!("enum two keys {:?}", simd_json::from_slice::<E>(&mut b).map_err(|e|e.to_string()));
   let mut b=br#"{"U":null}"#.to_vec(); println!("enum unit obj {:?}", simd_json::from_slice::<E>(&mut b).map_err(|e|e.to_string()));
   let mut b=br#"[{"U":null},"U",{"N":1}]"#.to_vec(); println!("enum vec {:?}", simd_json::from_slice::<Vec<E>>(&mut b).map_err(|e|e.to_string()));
   let mut b=br#"[{"S":{"a":1,"zzz":[1,2,{"q":3}]}},{"N":1}]"#.to_vec(); println!("enum unknown field {:?}", simd_json::from_slice::<Vec<E>>(&mut b).map_err(|e|e.to_string()));
   let mut b=br#""N""#.to_vec(); println!("enum str with data {:?}", simd_json::from_slice::<E>(&mut b).map_err(|e|e.to_string()));
   let mut b=br#"300"#.to_vec(); println!("u8 ovf {:?}", simd_json::from_slice::<u8>(&mut b).map_err(|e|e.to_string()));
   let mut b=br#"-1"#.to_vec(); println!("u8 neg {:?}", simd_json::from_slice::<u8>(&mut b).map_err(|e|e.to_string()));
   let mut b=br#"1.5"#.to_vec(); println!("u8 float {:?}", simd_json::from_slice::<u8>(&mut b).map_err(|e|e.to_string()));
   let mut b=br#"18446744073709551615"#.to_vec(); println!("f64 from u64 {:?} i128 {:?}", simd_json::from_slice::<f64>(&mut b.clone()).map_err(|e|e.to_string()), simd_json::from_slice::<i128>(&mut b).map_err(|e|e.to_string()));
   let mut b=br#"1e40"#.to_vec(); println!("f32 ovf {:?}", simd_json::from_slice::<f32>(&mut b).map_err(|e|e.to_string()));
   let mut b=br#"[]"#.to_vec(); println!("u8 arr {:?}", simd_json::from_slice::<u8>(&mut b).map_err(|e|e.to_string()));
   let mut b=br#"null"#.to_vec(); println!("opt {:?} unit", simd_json::from_slice::<Option<u8>>(&mut b).map_err(|e|e.to_string()));
   let mut b=br#"{"a":1}"#.to_vec(); println!("hm i key err {:?}", simd_json::from_slice::<HashMap<u8,u8>>(&mut b).map_err(|e|e.to_string()));
   let mut b=br#"{"1":1}"#.to_vec(); println!("hm i8 key {:?}", simd_json::from_slice::<HashMap<i8,u8>>(&mut b).map_err(|e|e.to_string()));
   let mut b=br#"{"true":1}"#.to_vec(); println!("hm bool key {:?}", simd_json::from_slice::<HashMap<bool,u8>>(&mut b).map_err(|e|e.to_string()));
   let mut b=br#"{"1.5":1}"#.to_vec(); println!("hm f64 key ser/de sj={:?}", serde_json::from_slice::<HashMap<String,u8>>(&b).is_ok()); let _=&mut b;
   // error eq
   println!("eq InputTooLarge {} Simd {} Unexpected {}", simd_json::ErrorType::InputTooLarge==simd_json::ErrorType::InputTooLarge, simd_json::ErrorType::SimdUnsupported==simd_json::ErrorType::SimdUnsupported, simd_json::ErrorType::Unexpected(None,None)==simd_json::ErrorType::Unexpected(None,None));
   let e=|s:&str|{let mut b=s.as_bytes().to_vec(); simd_json::to_owned_value(&mut b).unwrap_err()};
   for s in ["tru","[truX]","nul","[1","{\"a\":1","\"abc"] { let er=e(s); println!("{s:?} {:?} syntax={} eof={} data={} io={}", er.error(), er.is_syntax(), er.is_eof(), er.is_data(), er.is_io()); }
   let b5=char::from(0x5cu8);
   for s in [format!("\"{b5}ud800{b5}ue000\""),format!("\"{b5}ud800{b5}uffff\""),format!("\"{b5}udbff{b5}uffff\""),format!("\"{b5}ud800{b5}u0041\""),format!("\"{b5}ud800{b5}udc00\""),format!("\"{b5}ud83d{b5}ude00\""),format!("\"{b5}ud800{b5}n\""),format!("\"{b5}ud800{b5}uZZZZ\""),format!("\"{b5}u0000\""),format!("\"{b5}uD8\""),format!("\"{b5}u12\""),format!("\"x{b5}ud800\"")] { let mut bb=s.clone().into_bytes(); println!("{s} -> {:?} | sj {:?}", simd_json::to_owned_value(&mut bb).map(|v|v.as_str().map(|s|s.to_string())).map_err(|e|e.to_string()), serde_json::from_str::<String>(&s).map_err(|e|e.to_string())); }
  }
 }
}
fn serde_bytes_like()->Vec<u8>{vec![1,2]}
