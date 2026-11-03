use serde::{Serialize,Deserialize};
use simd_json::prelude::*;
struct FS(Vec<u8>,bool); impl Serialize for FS{fn serialize<S:serde::Serializer>(&self,s:S)->Result<S::Ok,S::Error>{ let k=self.1; s.collect_seq(self.0.iter().filter(move |_|k)) }}
struct FM(Vec<(u8,u8)>,bool); impl Serialize for FM{fn serialize<S:serde::Serializer>(&self,s:S)->Result<S::Ok,S::Error>{ let k=self.1; s.collect_map(self.0.iter().filter(move |_|k).map(|(a,b)|(a,b))) }}
#[derive(Serialize)] enum E{U,N(u8),T(u8,u8),T0(),S{a:u8},S0{},NN(Vec<u8>),TT(Vec<u8>,Vec<u8>)}
#[derive(Serialize)] struct W{e:E,x:u8}
#[derive(Serialize)] struct WF{a:FS,b:u8}
fn t<T:Serialize>(n:&str,v:&T){ let a=simd_json::to_string(v).map_err(|e|e.to_string()); let p=simd_json::to_string_pretty(v).map_err(|e|e.to_string()); let sa=serde_json::to_string(v).ok(); let sp=serde_json::to_string_pretty(v).ok();
 let valid=|s:&Result<String,String>|s.as_ref().ok().map(|x|serde_json::from_str::<serde_json::Value>(x).is_ok());
 println!("{n}: compact={a:?} valid={:?} same={} | pretty={p:?} valid={:?} same={}",valid(&a),a.as_ref().ok()==sa.as_ref(),valid(&p),p.as_ref().ok()==sp.as_ref()); }
fn main(){
 t("filt seq empty",&FS(vec![1,2],false)); t("filt seq",&FS(vec![1,2],true)); t("filt map empty",&FM(vec![(1,2)],false)); t("filt map",&FM(vec![(1,2)],true));
 t("in struct",&WF{a:FS(vec![1],false),b:2}); t("vec of filt",&vec![FS(vec![1],false),FS(vec![1],true)]);
 t("W U",&W{e:E::U,x:1});t("W N",&W{e:E::N(1),x:1});t("W T",&W{e:E::T(1,2),x:1});t("W T0",&W{e:E::T0(),x:1});t("W S",&W{e:E::S{a:1},x:1});t("W S0",&W{e:E::S0{},x:1});t("NN",&E::NN(vec![1,2]));t("TT",&E::TT(vec![1],vec![]));
 t("nested vec",&vec![vec![1],vec![]]);t("bytes",&B(vec![1,2]));t("bytes empty",&B(vec![]));
 t("map of T",&std::collections::BTreeMap::from([("a",E::T(1,2))]));
 // value keys escaping
 let doc=r#"{"a\"b":1,"c\\d":2,"e\nf":3,"g\u0001h":4,"é😀":5,"":6,"k":"v\"\\\n\u0001é"}"#;
 let want:serde_json::Value=serde_json::from_str(doc).unwrap();
 let chk=|n:&str,s:String|{ let ok=serde_json::from_str::<serde_json::Value>(&s).map(|v|v==want); println!("{n}: {s} => {:?}",ok.map_err(|e|e.to_string())); };
 let mut b=doc.as_bytes().to_vec(); let v=simd_json::to_owned_value(&mut b).unwrap(); chk("owned encode",v.encode()); chk("owned pp",v.encode_pp()); let mut w=vec![]; v.write(&mut w).unwrap(); chk("owned write",String::from_utf8(w).unwrap()); let mut w=vec![]; v.write_pp(&mut w).unwrap(); chk("owned write_pp",String::from_utf8(w).unwrap()); chk("owned serde",simd_json::to_string(&v).unwrap());chk("owned serde pp",simd_json::to_string_pretty(&v).unwrap());
 let mut b=doc.as_bytes().to_vec(); let v=simd_json::to_borrowed_value(&mut b).unwrap(); chk("borrowed encode",v.encode()); chk("borrowed pp",v.encode_pp()); let mut w=vec![]; v.write(&mut w).unwrap(); chk("borrowed write",String::from_utf8(w).unwrap());let mut w=vec![]; v.write_pp(&mut w).unwrap(); chk("borrowed write_pp",String::from_utf8(w).unwrap());
 let mut b=doc.as_bytes().to_vec(); let t_=simd_json::to_tape(&mut b).unwrap(); let tv=t_.as_value(); chk("tape encode",tv.encode()); chk("tape pp",tv.encode_pp()); let mut w=vec![]; tv.write(&mut w).unwrap(); chk("tape write",String::from_utf8(w).unwrap());let mut w=vec![]; tv.write_pp(&mut w).unwrap(); chk("tape write_pp",String::from_utf8(w).unwrap());
 let l=simd_json::value::lazy::Value::from_tape(tv); chk("lazy encode",l.encode()); chk("lazy pp",l.encode_pp());
 // nested pretty containers & empties
 let doc=r#"{"a":[],"b":{},"c":[[],{}],"d":[{"e":[1,{"f":null}]}]}"#; let want2:serde_json::Value=serde_json::from_str(doc).unwrap();
 let mut b=doc.as_bytes().to_vec(); let v=simd_json::to_owned_value(&mut b).unwrap(); println!("{}",v.encode_pp()); println!("eq {:?}",serde_json::from_str::<serde_json::Value>(&v.encode_pp()).map(|x|x==want2));
 let mut b=doc.as_bytes().to_vec(); let t_=simd_json::to_tape(&mut b).unwrap(); let pp=t_.as_value().encode_pp(); println!("{pp}"); println!("eq {:?}",serde_json::from_str::<serde_json::Value>(&pp).map(|x|x==want2)); println!("tape enc {} eq {:?}",t_.as_value().encode(),serde_json::from_str::<serde_json::Value>(&t_.as_value().encode()).map(|x|x==want2));
 let l=simd_json::value::lazy::Value::from_tape(t_.as_value()); println!("lazy arr {:?} obj keys {:?}", l.get("d").map(|d|d.is_array()), l.as_object().map(|o|o.keys().map(|k|k.to_string()).collect::<Vec<_>>()));
 println!("lazy cmp: {}", simd_json::value::lazy::Value::from_tape(t_.as_value())==simd_json::value::lazy::Value::from_tape(t_.as_value()));
 let mut up=simd_json::value::lazy::Value::from_tape(t_.as_value()); up.insert("zz",1).unwrap(); println!("lazy after {}",up.encode());
 // tape equality for equal docs w/ reordered keys; tape vs value eq
 let mut a=br#"{"a":1,"b":2}"#.to_vec(); let mut c=br#"{"b":2,"a":1}"#.to_vec(); let ta=simd_json::to_tape(&mut a).unwrap(); let tc=simd_json::to_tape(&mut c).unwrap(); println!("tape reorder eq {}",ta.as_value()==tc.as_value());
 let mut a=br#"{"a":1,"b":2}"#.to_vec(); let mut c=br#"{"b":2,"a":1}"#.to_vec(); let oa=simd_json::to_owned_value(&mut a).unwrap(); let oc=simd_json::to_owned_value(&mut c).unwrap(); println!("owned reorder eq {}",oa==oc);
 let mut a=b"[1.0]".to_vec(); let mut c=b"[1]".to_vec(); println!("1.0==1 owned {}",simd_json::to_owned_value(&mut a).unwrap()==simd_json::to_owned_value(&mut c).unwrap());
 let mut a=b"-1".to_vec(); let ov=simd_json::to_owned_value(&mut a).unwrap(); let mut a2=b"-1".to_vec(); let t2=simd_json::to_tape(&mut a2).unwrap(); let tv2=t2.as_value();println!("tape as_i32 {:?} as_u32 {:?} as_usize {:?} f32 {:?} cast {:?} / owned {:?}",tv2.as_i32(),tv2.as_u32(),tv2.as_usize(),tv2.as_f32(),tv2.cast_f64(),ov.as_i32());
 let mut a=b"[1,2]".to_vec(); let t3=simd_json::to_tape(&mut a).unwrap(); println!("len {:?} type {:?} {:?}",t3.as_value().as_array().map(|x|x.len()),t3.as_value().value_type(),t3.as_value().get_idx(5).is_none());
}
struct B(Vec<u8>); impl Serialize for B{fn serialize<S:serde::Serializer>(&self,s:S)->Result<S::Ok,S::Error>{s.serialize_bytes(&self.0)}}
#[allow(unused)] #[derive(Deserialize)] struct Nothing;
