#![allow(unused)]
use simd_json::prelude::*;
use simd_json::{OwnedValue, BorrowedValue, to_owned_value, to_borrowed_value, to_tape};
use serde::{Serialize, Deserialize, Serializer};
use std::collections::HashMap;

fn own(s: &[u8]) -> String { let mut b = s.to_vec(); format!("{:?}", to_owned_value(&mut b)) }
fn sj(s: &[u8]) -> String { match serde_json::from_slice::<serde_json::Value>(s) { Ok(v)=>format!("{:?}",v), Err(e)=>format!("ERR {}",e) } }
fn cmp(s: &[u8]) { println!("  {:?}\n    simd={}\n    sj  ={}", String::from_utf8_lossy(s), own(s), sj(s)); }
fn pad(n: usize, s: &str) -> Vec<u8> { let mut v = vec![b' '; n]; v.extend_from_slice(s.as_bytes()); v }

#[derive(Serialize, Deserialize, Debug, PartialEq)]
enum E { T(u8,u8), T0(), N(u8), S{a:u8,b:u8}, U }
#[derive(Serialize, Deserialize, Debug)]
struct P { a:u8, b:u8 }
#[derive(Deserialize, Debug)]
struct T2(u8,u8);

fn try_<F: FnOnce()->String + std::panic::UnwindSafe>(name:&str, f:F) {
    match std::panic::catch_unwind(f) { Ok(s)=>println!("  {name}: {s}"), Err(e)=>println!("  {name}: PANIC {:?}", e.downcast_ref::<String>().map(|s|s.as_str()).or(e.downcast_ref::<&str>().copied())) }
}

fn numbers() {
    println!("== long float offset");
    cmp(br#"{"sensor":"a1","time":1700000000,"unit":"degreesN","lat":48.858369400000000000012}"#);
    cmp(br#"{"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa":0.21575530329307963391}"#);
    cmp(br#"[0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,1.1,1.2,1.3,123456789012345678901.5]"#);
    let mut bad=0; for off in 0..200 { let v=pad(off,"1.2345678901234567890123"); let mut b=v.clone(); let r=to_owned_value(&mut b).unwrap().as_f64().unwrap(); if (r-1.2345678901234567).abs()>1e-9 {bad+=1; if bad<3 {println!("  off {off}: {r}");}} }
    println!("  bad offsets {bad}/200");
    println!("== exponent wrap");
    for s in ["1e4294967296","1e4294967297","1e4294967295","1e-4294967296","1.5e4294967296","1e-4294967297","1.5e4294967298","1.5e4294967300","1e400"] { cmp(s.as_bytes()); }
    println!("== big exponents rejected");
    for s in ["0e999999999999","0e99999999999","1e-99999999999","1e-400"] { cmp(s.as_bytes()); }
    println!("== trailing garbage");
    for s in ["[1.23456789012345678901234x]","[12345678901234567890x]","[1234567890123456789abc]","[975758694.12340, false,383408244.24957958631607.24113]","12345678901234567890abc","1.00000000000000000000000x","[15x]","[1.5x]"] { cmp(s.as_bytes()); }
    println!("== NUL");
    for s in [&b"[1\0garbage]"[..], b"1\0garbage", b"true\0garbage", b"{\"a\":null\0zzz}", b"[1.5\0x,2]", b"[1\0,2]", b"84.544\x004", b"\"a\"\0", b"123\0"] { cmp(s); }
    println!("== -0");
    for s in ["-0","-0.0","[-0]"] { cmp(s.as_bytes()); }
}
fn strings() {
    println!("== surrogates");
    for s in [r#""\ud800""#, r#""\ud800x""#, r#""\ud83dabc""#, r#"{"\ud800":1}"#, r#""\ud800""#, r#""\ud800￿""#, r#""\udbff￿""#, r#""\udc00""#, r#""\ud800A""#] { cmp(s.as_bytes()); }
    // on a short string via from_slice<String> and tape
    let mut b=br#""\ud800""#.to_vec(); println!("  from_slice String {:?}", simd_json::from_slice::<String>(&mut b));
}
fn tape() {
    println!("== tape get_array/get_object");
    let mut b=br#"{"a":[1,2,3],"o":{"x":1,"y":2},"n":[[1,2]],"p":{"k":[1,2]},"e":[],"f":{}}"#.to_vec();
    let t=to_tape(&mut b).unwrap(); let v=t.as_value();
    let a=v.get_array("a").unwrap();
    println!("  len={} get2={:?} iter={:?}", a.len(), a.get(2).map(|x|x.as_u64()), a.iter().map(|x|x.as_u64()).collect::<Vec<_>>());
    println!("  via get: {:?}", v.get("a").unwrap().as_array().map(|a|a.iter().count()));
    let o=v.get_object("o").unwrap();
    println!("  obj.get(y)={:?} get(x)={:?}", o.get("y").map(|x|x.as_u64()), o.get("x").map(|x|x.as_u64()));
    try_("nested arr iter", ||{let mut b=br#"{"n":[[1,2]]}"#.to_vec();let t=to_tape(&mut b).unwrap();let v=t.as_value();let c=v.get_array("n").unwrap().iter().count();format!("count {c}")});
    try_("nested obj", ||{let mut b=br#"{"p":{"k":[1,2]}}"#.to_vec();let t=to_tape(&mut b).unwrap();let v=t.as_value();let o=v.get_object("p").unwrap();let c=o.iter().count();format!("count {c}")});
    try_("empty arr len", ||{let mut b=br#"{"e":[]}"#.to_vec();let t=to_tape(&mut b).unwrap();let v=t.as_value();let c=v.get_array("e").unwrap().len();format!("len {c}")});
    try_("empty obj len", ||{let mut b=br#"{"f":{}}"#.to_vec();let t=to_tape(&mut b).unwrap();let v=t.as_value();let c=v.get_object("f").unwrap().len();format!("len {c}")});
    try_("try_get_array", ||{let mut b=br#"{"a":[1,2,3]}"#.to_vec();let t=to_tape(&mut b).unwrap();let v=t.as_value();let a=v.try_get_array("a").unwrap().unwrap();format!("len {} iter {}",a.len(),a.iter().count())});
    try_("lazy get_array", ||{let mut b=br#"{"a":[1,2,3]}"#.to_vec();let t=to_tape(&mut b).unwrap();let l=simd_json::value::lazy::Value::from_tape(t.as_value());let a=l.get_array("a").unwrap();format!("len {} iter {}",a.len(),a.iter().count())});
    println!("== equality");
    let mut b1=br#"{"a":1,"b":2}"#.to_vec(); let mut b2=br#"{"b":2,"a":1}"#.to_vec();
    let t1=to_tape(&mut b1.clone()).map(|t|format!("{:?}",t)).unwrap();
    { let mut x=b1.clone(); let mut y=b2.clone(); let t1=to_tape(&mut x).unwrap(); let t2=to_tape(&mut y).unwrap(); println!("  tape reordered == : {}", t1.as_value()==t2.as_value()); }
    { let mut x=b1.clone(); let mut y=b2.clone(); println!("  owned reordered == : {}", to_owned_value(&mut x).unwrap()==to_owned_value(&mut y).unwrap()); }
    { let mut x=b1.clone(); let t=to_tape(&mut x).unwrap(); let l1=simd_json::value::lazy::Value::from_tape(t.as_value()); let mut l2=simd_json::value::lazy::Value::from_tape(t.as_value()); let _=l2.as_object_mut(); println!("  lazy tape vs upgraded: is_value={} eq={}", l2.is_value(), l1==l2); }
    println!("== display");
    let mut x=br#"{"a":[1,"x"],"b":null}"#.to_vec(); println!("  owned {}", to_owned_value(&mut x).unwrap());
    let mut x=br#"{"a":[1,"x"],"b":null}"#.to_vec(); println!("  borrowed {}", to_borrowed_value(&mut x).unwrap());
    let mut x=br#"{"a":[1,"x"],"b":null}"#.to_vec(); let t=to_tape(&mut x).unwrap(); println!("  lazy {}", simd_json::value::lazy::Value::from_tape(t.as_value()));
    println!("== reset / fill tape");
    try_("reset value_type", ||{let t=simd_json::tape::Tape::null().reset(); format!("{:?}", t.as_value().value_type())});
    try_("reset encode", ||{let t=simd_json::tape::Tape::null().reset(); format!("{:?}", t.as_value().encode())});
    {
        let mut buf=simd_json::Buffers::default(); let mut t=simd_json::tape::Tape::null();
        let mut x=b"[1,".to_vec();
        let r=simd_json::fill_tape(&mut x,&mut buf,&mut t);
        println!("  fill_tape {:?} tape={:?}", r.as_ref().map_err(|e|e.to_string()), t.0);
        let tv=t.as_value(); println!("  encode={}", tv.encode());
    }
}
fn serde_de() {
    println!("== surplus");
    fn d<'a, T: Deserialize<'a>+std::fmt::Debug>(s:&'a mut Vec<u8>)->String { format!("{:?}", simd_json::from_slice::<T>(s).map_err(|e|e.to_string())) }
    println!("  {}", d::<Vec<(u8,u8)>>(&mut br#"[[1,2,[3,4]],[5,6]]"#.to_vec()));
    println!("  {}", d::<(u8,u8)>(&mut br#"[1,2,3]"#.to_vec()));
    println!("  {}", d::<[u8;2]>(&mut br#"[1,2,3]"#.to_vec()));
    println!("  {}", d::<P>(&mut br#"[1,2,3]"#.to_vec()));
    println!("  {}", d::<E>(&mut br#"{"T":[1,2,[7,8]]}"#.to_vec()));
    println!("  {}", d::<Vec<(u8,)>>(&mut br#"[[1,[7]],[3,4]]"#.to_vec()));
    println!("  {}", d::<((u8,),u8)>(&mut br#"[[1,2],3]"#.to_vec()));
    println!("  sj {:?}", serde_json::from_str::<(u8,u8)>("[1,2,3]").map_err(|e|e.to_string()));
    let mut x=br#"[1,2,3]"#.to_vec(); let v=to_owned_value(&mut x).unwrap(); println!("  from_owned {:?}", simd_json::serde::from_owned_value::<(u8,u8)>(v).map_err(|e|e.to_string()));
    println!("== trailing doc items? from_slice does not check end of tape");
    println!("== int keys by value");
    let mut x=br#"{"1":2}"#.to_vec(); let v=to_owned_value(&mut x).unwrap();
    println!("  refowned {:?}", simd_json::serde::from_refowned_value::<HashMap<u8,u8>>(&v).map_err(|e|e.to_string()));
    println!("  owned {:?}", simd_json::serde::from_owned_value::<HashMap<u8,u8>>(v).map_err(|e|e.to_string()));
    let mut x=br#"{"1":2}"#.to_vec(); let v=to_borrowed_value(&mut x).unwrap();
    println!("  refborrowed {:?}", simd_json::serde::from_refborrowed_value::<HashMap<u8,u8>>(&v).map_err(|e|e.to_string()));
    println!("  borrowed {:?}", simd_json::serde::from_borrowed_value::<HashMap<u8,u8>>(v).map_err(|e|e.to_string()));
    println!("== BorrowedValue keys");
    println!("  sj->Borrowed {:?}", serde_json::from_str::<BorrowedValue>(r#"{"a\nb":1}"#).map_err(|e|e.to_string()));
    println!("  sj->Owned {:?}", serde_json::from_str::<OwnedValue>(r#"{"a\nb":1}"#).map_err(|e|e.to_string()));
    let mut x=br#"{"a\nb":1}"#.to_vec(); println!("  simd from_slice->Borrowed {:?}", simd_json::from_slice::<BorrowedValue>(&mut x).map_err(|e|e.to_string()));
    let mut x=br#"{"a\nb":1}"#.to_vec(); let o=to_borrowed_value(&mut x).unwrap(); println!("  from_borrowed->Borrowed {:?}", simd_json::serde::from_borrowed_value::<BorrowedValue>(o).map_err(|e|e.to_string()));
    println!("== empty tuple variant via values");
    let o=simd_json::serde::to_owned_value(&E::T0()); println!("  to_owned {:?}", o);
    println!("  back {:?}", simd_json::serde::from_owned_value::<E>(o.unwrap()).map_err(|e|e.to_string()));
    let o=simd_json::serde::to_owned_value(&E::T0()).unwrap(); println!("  ref back {:?}", simd_json::serde::from_refowned_value::<E>(&o).map_err(|e|e.to_string()));
    let mut x=br#"{"T0":[]}"#.to_vec(); println!("  from_slice {:?}", simd_json::from_slice::<E>(&mut x).map_err(|e|e.to_string()));
    let mut x=br#"{"T0":[]}"#.to_vec(); let o=to_borrowed_value(&mut x).unwrap(); println!("  from_borrowed {:?}", simd_json::serde::from_borrowed_value::<E>(o).map_err(|e|e.to_string()));
    println!("== map keys");
    let mut x=br#"{"true":1}"#.to_vec(); println!("  bool from_slice {:?}", simd_json::from_slice::<HashMap<bool,u8>>(&mut x).map_err(|e|e.to_string()));
    println!("  sj bool {:?}", serde_json::from_str::<HashMap<bool,u8>>(r#"{"true":1}"#).map_err(|e|e.to_string()));
    let mut h=HashMap::new(); h.insert(true,1u8); println!("  bool to_string {:?} sj {:?}", simd_json::to_string(&h).map_err(|e|e.to_string()), serde_json::to_string(&h).map_err(|e|e.to_string()));
    let mut x=br#"{"1":1}"#.to_vec(); println!("  i128 key from_slice {:?}", simd_json::from_slice::<HashMap<i128,u8>>(&mut x).map_err(|e|e.to_string()));
    let mut x=br#"{"1":1}"#.to_vec(); println!("  u128 key from_slice {:?}", simd_json::from_slice::<HashMap<u128,u8>>(&mut x).map_err(|e|e.to_string()));
    let mut h=HashMap::new(); h.insert(1u128,1u8); println!("  u128 key to_string {:?}; to_owned {:?}", simd_json::to_string(&h).map_err(|e|e.to_string()), simd_json::serde::to_owned_value(&h).map_err(|e|e.to_string()));
    let mut x=b"18446744073709551615".to_vec(); println!("  max u64 as i128 {:?}", simd_json::from_slice::<i128>(&mut x).map_err(|e|e.to_string()));
    println!("== dup keys");
    let mut x=br#"{"a":1,"a":2}"#.to_vec(); let v=to_owned_value(&mut x).unwrap();
    println!("  len {} get {:?} enc {} eqclone {}", v.as_object().unwrap().len(), v.get("a"), v.encode(), v==v.clone());
    let mut x=br#"{"a":1,"a":2}"#.to_vec(); println!("  from_slice Owned {:?}", simd_json::from_slice::<OwnedValue>(&mut x).map(|v|v.encode()));
    let mut v2=v.clone(); let _=v2.as_object_mut().unwrap().remove("a"); println!("  after remove {:?}", v2.get("a"));
}
fn serde_se() {
    println!("== pretty tuple variant");
    println!("  {:?}", simd_json::to_string_pretty(&E::T(1,2)));
    println!("  {:?}", simd_json::to_string_pretty(&E::T0()));
    println!("  compact {:?}", simd_json::to_string(&E::T(1,2)));
    println!("  sibl {:?}", simd_json::to_string_pretty(&vec![E::T(1,2),E::U]));
    println!("  sj {:?}", serde_json::to_string_pretty(&E::T(1,2)).unwrap());
    println!("== empty unknown-length");
    struct Filt; impl Serialize for Filt { fn serialize<S:Serializer>(&self,s:S)->Result<S::Ok,S::Error>{ s.collect_seq([1u8].iter().filter(|_|false)) } }
    struct FiltM; impl Serialize for FiltM { fn serialize<S:Serializer>(&self,s:S)->Result<S::Ok,S::Error>{ s.collect_map([(1u8,2u8)].iter().filter(|_|false).map(|x|(x.0.to_string(),x.1))) } }
    struct ZStruct; impl Serialize for ZStruct { fn serialize<S:Serializer>(&self,s:S)->Result<S::Ok,S::Error>{ use serde::ser::SerializeStruct; let st=s.serialize_struct("x",2)?; st.end() } }
    struct Z0Struct; impl Serialize for Z0Struct { fn serialize<S:Serializer>(&self,s:S)->Result<S::Ok,S::Error>{ use serde::ser::SerializeStruct; let st=s.serialize_struct("x",0)?; st.end() } }
    struct SomeSeq; impl Serialize for SomeSeq { fn serialize<S:Serializer>(&self,s:S)->Result<S::Ok,S::Error>{ use serde::ser::SerializeSeq; let st=s.serialize_seq(Some(3))?; st.end() } }
    #[derive(Serialize)] struct W{items:Filt,next:u8}
    #[derive(Serialize)] struct Fl{#[serde(flatten)] m:HashMap<String,u8>}
    println!("  seq compact {:?} pretty {:?} sj {:?}", simd_json::to_string(&Filt), simd_json::to_string_pretty(&Filt), serde_json::to_string(&Filt));
    println!("  map compact {:?} pretty {:?} sj {:?}", simd_json::to_string(&FiltM), simd_json::to_string_pretty(&FiltM), serde_json::to_string(&FiltM));
    println!("  in struct {:?}", simd_json::to_string(&W{items:Filt,next:2}));
    println!("  zstruct(2) {:?} {:?}", simd_json::to_string(&ZStruct), simd_json::to_string_pretty(&ZStruct));
    println!("  zstruct(0) {:?} {:?}", simd_json::to_string(&Z0Struct), simd_json::to_string_pretty(&Z0Struct));
    println!("  flatten empty {:?} {:?}", simd_json::to_string(&Fl{m:HashMap::new()}), simd_json::to_string_pretty(&Fl{m:HashMap::new()}));
    println!("  someseq(3) {:?} {:?}", simd_json::to_string(&SomeSeq), simd_json::to_string_pretty(&SomeSeq));
    println!("  someseq in vec pretty {:?}", simd_json::to_string_pretty(&vec![SomeSeq]));
    println!("== nan");
    println!("  {:?} {:?}", simd_json::to_string(&vec![f64::NAN,f64::INFINITY,f64::NEG_INFINITY]), serde_json::to_string(&vec![f64::NAN,f64::INFINITY]));
    println!("  {:?}", simd_json::to_string_pretty(&f64::NAN));
    println!("  encode {:?}", OwnedValue::from(f64::NAN).encode());
    println!("  f32 nan {:?}", simd_json::to_string(&f32::NAN));
    let mut x=b"2.696539702293474e308".to_vec(); println!("  reparse {:?}", to_owned_value(&mut x).map_err(|e|e.to_string()));
    println!("== f32");
    println!("  {:?} sj {:?}", simd_json::to_string(&0.1f32), serde_json::to_string(&0.1f32));
    println!("== pretty layout");
    println!("  struct {:?}", simd_json::to_string_pretty(&P{a:1,b:2}));
    println!("  sv {:?}", simd_json::to_string_pretty(&E::S{a:1,b:2}));
    println!("  newtype {:?}", simd_json::to_string_pretty(&E::N(1)));
    struct By; impl Serialize for By { fn serialize<S:Serializer>(&self,s:S)->Result<S::Ok,S::Error>{ s.serialize_bytes(&[1,2,3]) } }
    println!("  bytes {:?} compact {:?} sj {:?}", simd_json::to_string_pretty(&By), simd_json::to_string(&By), serde_json::to_string_pretty(&By));
}
fn errors() {
    use simd_json::ErrorType as T;
    println!("== error eq");
    println!("  {} {} {} {}", T::InputTooLarge==T::InputTooLarge, T::SimdUnsupported==T::SimdUnsupported, T::Unexpected(None,None)==T::Unexpected(None,None), T::Eof==T::Eof);
    println!("  io eq {}", T::Io(std::io::Error::new(std::io::ErrorKind::NotFound,"x"))==T::Io(std::io::Error::new(std::io::ErrorKind::Other,"y")));
    println!("== error classes");
    for s in ["tru","nul","[tru]","truex","[1,","\"abc","{\"a\":1","[1","[] x","[1] 2","[]]","1 2","[1,]","]","+1","x","[1]x","{}x","{\"a\":}"] {
        let mut b=s.as_bytes().to_vec();
        match to_owned_value(&mut b) { Ok(v)=>println!("  {s:?} OK {v:?}"), Err(e)=>println!("  {s:?} {:?} syn={} eof={} data={} idx={} ch={:?}", e.error(), e.is_syntax(), e.is_eof(), e.is_data(), e.index(), e.character()) }
    }
    println!("== error index");
    for s in [&br#"            "abc\q""#[..], br#"[1,2,"\x"]"#, br#"[1,2,3,4,5,6,"abc\uZZZZ"]"#, b"[    \"\x01\"]", b"\xef\xbb\xbf1", b"[1,\xff]"] {
        let mut b=s.to_vec();
        match to_owned_value(&mut b) { Ok(v)=>println!("  {:?} OK {v:?}", String::from_utf8_lossy(s)), Err(e)=>println!("  {:?} {:?} idx={} ch={:?}", String::from_utf8_lossy(s), e.error(), e.index(), e.character()) }
    }
    println!("== source");
    let e:simd_json::Error = std::io::Error::new(std::io::ErrorKind::Other,"y").into();
    println!("  source {:?}", std::error::Error::source(&e).is_some());
    println!("== buffers");
    
}
fn ub() {
    println!("== double deser");
    let mut b=b"1".to_vec(); let mut d=simd_json::Deserializer::from_slice(&mut b).unwrap();
    println!("  first {:?}", u8::deserialize(&mut d).map_err(|e|e.to_string()));
    println!("  second {:?}", u8::deserialize(&mut d).map_err(|e|e.to_string()));
}
fn ub2() { println!("  {:?}", simd_json::tape::Tape::null().reset().deserialize::<u8>().map_err(|e|e.to_string())); }
fn ub3() {
    use simd_json::tape::{Tape,Node};
    let t=Tape(vec![Node::Array{len:3,count:3}]); let v=simd_json::value::lazy::Value::from_tape(t.as_value()).into_value();
    println!("  {:?}", v);
}
fn deep(kind:&str, n:usize) {
    let mut b = if kind.ends_with("obj") { let mut v=Vec::new(); for _ in 0..n {v.extend_from_slice(b"{\"a\":");} v.extend_from_slice(b"1"); for _ in 0..n {v.push(b'}');} v } else { let mut v=vec![b'[';n]; v.extend(vec![b']';n]); v };
    match kind {
        "owned"|"ownedobj" => println!("ok {}", to_owned_value(&mut b).is_ok()),
        "borrowed" => println!("ok {}", to_borrowed_value(&mut b).is_ok()),
        "de" => println!("ok {}", simd_json::from_slice::<OwnedValue>(&mut b).is_ok()),
        "ign" => println!("ok {}", simd_json::from_slice::<serde::de::IgnoredAny>(&mut b).is_ok()),
        "tape" => println!("ok {}", to_tape(&mut b).is_ok()),
        "encode" => { let t=to_tape(&mut b).unwrap(); println!("ok {}", t.as_value().encode().len()) },
        _=>panic!(),
    }
}
fn sur() {
    let bs=char::from(92);
    for (a,b) in [("d800","e000"),("d800","ffff"),("dbff","ffff"),("d800","0041"),("d800","d800"),("d83d","de00")] {
        let s=format!("\"{bs}u{a}{bs}u{b}\""); let r=own(s.as_bytes()); println!("  {s} simd={r} sj={}", sj(s.as_bytes()));
    }
    println!("  std cow");
    let mut x=br#"{"k":1}"#.to_vec(); let mut o=to_borrowed_value(&mut x).unwrap();
    o.as_object_mut().unwrap().insert("own".to_string().into(), BorrowedValue::from(1));
    println!("  from_borrowed owned key {:?}", simd_json::serde::from_borrowed_value::<BorrowedValue>(o).map_err(|e|e.to_string()));
    println!("  dup big");
    let mut d=String::from("{"); for i in 0..40 { d+=&format!("\"k{i}\":{i},"); } d+="\"k0\":99}";
    let mut b=d.clone().into_bytes(); let mut v=to_owned_value(&mut b).unwrap(); let m=v.as_object_mut().unwrap(); println!("  len {} get {:?}", m.len(), m.get("k0")); m.remove("k0"); println!("  after remove {:?} len {}", m.get("k0"), m.len());
    let mut b=d.clone().into_bytes(); println!("  hm {:?}", simd_json::from_slice::<HashMap<String,u8>>(&mut b).map(|h|h["k0"]));
    let mut b=br#"[1] 2"#.to_vec(); println!("  from_slice u8 trailing? {:?}", simd_json::from_slice::<Vec<u8>>(&mut b).map_err(|e|e.to_string()));
    println!("  uses into_owned? cow");
}
fn main() {
    let a:Vec<String>=std::env::args().collect();
    match a[1].as_str() {
        "all"=>{numbers();strings();tape();serde_de();serde_se();errors();}
        "sur"=>sur(),"ub"=>ub(),"ub2"=>ub2(),"ub3"=>ub3(),
        "deep"=>deep(&a[2],a[3].parse().unwrap()),
        _=>panic!(),
    }
}
