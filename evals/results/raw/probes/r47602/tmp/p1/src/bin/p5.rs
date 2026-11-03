use serde::{Deserialize, Serialize, Serializer};
use simd_json::prelude::*;
use simd_json::{OwnedValue, BorrowedValue};
use std::collections::{HashMap, BTreeMap};
struct EmptyMap;
impl Serialize for EmptyMap {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let v: Vec<(u8,u8)> = vec![(1, 2)];
        s.collect_map(v.iter().filter(|_| false).map(|x| (x.0, x.1)))
    }
}
#[derive(Serialize)]
struct Fl { #[serde(flatten)] m: BTreeMap<String,u8> }
#[derive(Serialize)]
struct Fl2 { a: u8, #[serde(flatten)] m: BTreeMap<String,u8> }
#[derive(Serialize, Deserialize, Debug, PartialEq)]
enum E { U, Tv(u8, u8), Sv { a: u8, b: u8 } }
#[derive(Serialize)]
struct Sk { #[serde(skip_serializing_if="Option::is_none")] a: Option<u8> }
fn main() {
    println!("compact empty collect_map {:?}", simd_json::to_string(&EmptyMap));
    println!("pretty  empty collect_map {:?}", simd_json::to_string_pretty(&EmptyMap));
    let f = Fl{m:BTreeMap::new()};
    println!("flatten empty compact {:?} pretty {:?} sj {:?}", simd_json::to_string(&f), simd_json::to_string_pretty(&f), serde_json::to_string_pretty(&f).ok());
    let f = Fl2{a:1,m:BTreeMap::from([("x".to_string(),2)])};
    println!("flatten  compact {:?} pretty {:?}", simd_json::to_string(&f), simd_json::to_string_pretty(&f));
    println!("skip all: {:?} {:?}", simd_json::to_string(&Sk{a:None}), simd_json::to_string_pretty(&Sk{a:None}));
    // nested indentation after tuple variant
    println!("pretty vec of enums {:?}", simd_json::to_string_pretty(&vec![E::Tv(1,2), E::U]));
    println!("pretty vec of sv {:?}", simd_json::to_string_pretty(&vec![E::Sv{a:1,b:2}, E::U]));
    // pretty roundtrip
    let mut s = simd_json::to_string_pretty(&E::Tv(1,2)).unwrap().into_bytes();
    println!("pretty Tv roundtrip {:?}", simd_json::from_slice::<E>(&mut s).map_err(|e|e.to_string()));
    let mut s = simd_json::to_string_pretty(&E::Sv{a:1,b:2}).unwrap().into_bytes();
    println!("pretty Sv roundtrip {:?}", simd_json::from_slice::<E>(&mut s).map_err(|e|e.to_string()));
    // Value-level int keys
    let mut b = br#"{"1":2}"#.to_vec();
    let ov = simd_json::to_owned_value(&mut b).unwrap();
    println!("HashMap<u8,u8> from &Owned: {:?}", HashMap::<u8,u8>::deserialize(&ov).map_err(|e|e.to_string()));
    println!("HashMap<u8,u8> from Owned : {:?}", HashMap::<u8,u8>::deserialize(ov).map_err(|e|e.to_string()));
    let mut b = br#"{"1":2}"#.to_vec();
    let bv = simd_json::to_borrowed_value(&mut b).unwrap();
    println!("HashMap<u8,u8> from &Borrowed: {:?}", HashMap::<u8,u8>::deserialize(&bv).map_err(|e|e.to_string()));
    println!("HashMap<u8,u8> from Borrowed : {:?}", HashMap::<u8,u8>::deserialize(bv).map_err(|e|e.to_string()));
    let sj: serde_json::Value = serde_json::from_str(r#"{"1":2}"#).unwrap();
    println!("HashMap<u8,u8> from serde_json::Value: {:?}", HashMap::<u8,u8>::deserialize(sj).map_err(|e|e.to_string()));
    // Value to_value round-trips
    let v = simd_json::serde::to_owned_value(E::Tv(1,2)).unwrap();
    println!("to_owned_value Tv {:?}", v.encode());
    println!("E from value {:?}", E::deserialize(v).map_err(|e|e.to_string()));
    let v = simd_json::serde::to_owned_value(&E::Sv{a:1,b:2}).unwrap();
    println!("E from value {:?}", E::deserialize(&v).map_err(|e|e.to_string()));
    let v = simd_json::serde::to_owned_value(&E::U).unwrap();
    println!("E from value {:?}", E::deserialize(&v).map_err(|e|e.to_string()));
    let v: BorrowedValue = simd_json::serde::to_borrowed_value(&E::Tv(3,4)).unwrap();
    println!("E from bvalue {:?}", E::deserialize(v).map_err(|e|e.to_string()));
    println!("to_owned_value bytes {:?}", simd_json::serde::to_owned_value(serde_bytes_like()).map(|v| v.encode()).map_err(|e| e.to_string()));
    // key types in to_value
    println!("i128 to_value {:?}", simd_json::serde::to_owned_value(1i128).map(|v| v.encode()).map_err(|e| e.to_string()));
    println!("i128 to_string {:?}", simd_json::to_string(&1i128).map_err(|e|e.to_string()));
    println!("i128 key to_string {:?}", simd_json::to_string(&BTreeMap::from([(1i128,1u8)])).map_err(|e|e.to_string()));
    println!("i128 key to_value {:?}", simd_json::serde::to_owned_value(&BTreeMap::from([(1i128,1u8)])).map(|v| v.encode()).map_err(|e|e.to_string()));
    let mut b = br#"{"1":2}"#.to_vec();
    println!("i128 key from_slice {:?}", simd_json::from_slice::<BTreeMap<i128,u8>>(&mut b).map_err(|e|e.to_string()));
    let mut b = br#"170141183460469231731687303715884105727"#.to_vec();
    println!("big int from_slice {:?}", simd_json::from_slice::<i128>(&mut b).map_err(|e|e.to_string()));
    let mut b = br#"5"#.to_vec();
    println!("i128 from_slice {:?}", simd_json::from_slice::<i128>(&mut b).map_err(|e|e.to_string()));
    let mut b = br#"-5"#.to_vec();
    println!("u128 from -5 {:?}", simd_json::from_slice::<u128>(&mut b).map_err(|e|e.to_string()));
    // from f64 with u64
    let mut b = b"18446744073709551615".to_vec();
    println!("f64 from u64max {:?}", simd_json::from_slice::<f64>(&mut b).map_err(|e|e.to_string()));
    // get_idx on non-array, Index panic
    let _ = OwnedValue::from(1);
    // tape value_type etc on empty tape
    let e = std::panic::catch_unwind(|| { let t = simd_json::Tape(vec![]); t.as_value().value_type() });
    println!("empty tape value_type {:?}", e.map_err(|_|"PANIC"));
    // restart
    let mut b = b"[1,2]".to_vec();
    let mut d = simd_json::Deserializer::from_slice(&mut b).unwrap();
    let a = Vec::<u8>::deserialize(&mut d);
    d.restart();
    let a2 = Vec::<u8>::deserialize(&mut d);
    println!("restart {:?} {:?}", a.ok(), a2.ok());
    // fill_tape error then use
    let mut buf = simd_json::Buffers::default();
    let mut tape = simd_json::Tape::null();
    let mut bad = b"[1,{\"a\":[2,".to_vec();
    println!("fill_tape err {:?}", simd_json::fill_tape(&mut bad, &mut buf, &mut tape).map_err(|e|e.to_string()));
    println!("tape after failure: {:?}", tape.0);
    println!("as_value encode {:?}", std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| tape.as_value().encode())).map_err(|_|"PANIC"));
    // from_slice input rewritten even on failure
    let mut b = br#"["a\n", tru]"#.to_vec();
    let r = simd_json::to_owned_value(&mut b);
    println!("after err {:?} buf={:?}", r.is_err(), String::from_utf8_lossy(&b));
    // input with escaped strings stays valid utf8? from_str
    let mut st = String::from(r#"{"ééé":"😀é"}"#.replace("%","\\"));
    let v: Result<OwnedValue,_> = unsafe { simd_json::from_str(&mut st) };
    println!("from_str {:?}, input valid utf8 after: {}", v.is_ok(), std::str::from_utf8(st.as_bytes()).is_ok());
    // Buffers shared
    let mut buffers = simd_json::Buffers::new(1);
    for s in ["[1]", "{\"a\":\"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\\n\"}", "2"] {
        let mut b = s.as_bytes().to_vec();
        println!("buffers: {:?}", simd_json::to_owned_value_with_buffers(&mut b, &mut buffers).map(|v|v.encode()).map_err(|e|e.to_string()));
    }
    // Eq between kinds
    println!("1==1.0 static? {}", OwnedValue::from(1u64)==OwnedValue::from(1.0f64));
    println!("U64(1)==I64(1)? {}", OwnedValue::from(1u64)==OwnedValue::from(1i64));
    println!("as_f32 big {:?}", OwnedValue::from(1e300f64).as_f32());
    println!("usize max {:?}", OwnedValue::from(u64::MAX).as_usize());
    // owned Eq with dup keys / object sizes
    // tape PartialEq object order
    let mut a = br#"{"a":1,"b":2}"#.to_vec();
    let mut b = br#"{"b":2,"a":1}"#.to_vec();
    let ta = simd_json::to_tape(&mut a).unwrap();
    let tb = simd_json::to_tape(&mut b).unwrap();
    println!("tape value eq reorder {}", ta.as_value()==tb.as_value());
    let mut a = br#"{"a":1,"b":2}"#.to_vec();
    let mut b = br#"{"b":2,"a":1}"#.to_vec();
    println!("owned eq reorder {}", simd_json::to_owned_value(&mut a).unwrap()==simd_json::to_owned_value(&mut b).unwrap());
    let _ = HashMap::<u8,u8>::new();
}
fn serde_bytes_like() -> Vec<u8> { vec![1,2] }
