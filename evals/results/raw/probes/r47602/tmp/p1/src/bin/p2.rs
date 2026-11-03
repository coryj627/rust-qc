use serde::Deserialize;
use simd_json::prelude::*;
use simd_json::{OwnedValue, BorrowedValue, Tape, Node, StaticNode};
fn own(s: &str) -> String {
    let mut b = s.as_bytes().to_vec();
    match simd_json::to_owned_value(&mut b) {
        Ok(v) => format!("OK {:?}", v),
        Err(e) => format!("ERR {}", e),
    }
}
fn sj(s: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(s) {
        Ok(v) => format!("OK {:?}", v),
        Err(e) => format!("ERR {}", e),
    }
}
fn u(s: &str) -> String { s.replace('%', "\\") }
fn show(s: &str) {
    let s = u(s);
    println!("input={s}\n   simd : {}\n   serde: {}", own(&s), sj(&s));
}
fn main() {
    for s in ["\"%uD800%uE000\"", "\"%uD800%uFFFF\"", "\"%uDBFF%uFFFF\"", "\"%uD800%u0041\"", "\"%uDC00\"", "\"%uDFFF\"", "\"%u12G4\"", "\"%uD800%u12G4\"", "\"%uD800%n\"", "\"%u12\"", "\"%uD83D%uDE00\"", "\"%uD83D\"", "\"%uD83Dabc\"", "\"%x\"", "\"%u0000\"", "\"%uFFFF\"", "\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa%uD800\"", "{\"%uD800\":1}"] {
        show(s);
    }
    let cases = ["[1,]", "{\"a\":1,}", "[1] x", "\"a\u{1}b\"", "\"a\u{7f}b\"", "\"\\", "\"\\\"", "[\"\\u", "\"\\u12"];
    for s in cases { println!("input={s:?}\n  simd: {}\n  serde: {}", own(s), sj(s)); }
    // tape get_array etc
    let mut b = br#"{"a":[1,2,3],"o":{"x":1,"y":2},"n":[[1,2]],"m":{"k":{"z":1}}}"#.to_vec();
    let t = simd_json::to_tape(&mut b).unwrap();
    let v = t.as_value();
    let arr = v.get_array("a").unwrap();
    println!("get_array a: len={} iter={:?} get2={:?}", arr.len(), arr.iter().map(|x| x.as_u64()).collect::<Vec<_>>(), arr.get(2).map(|x| x.as_u64()));
    let arr2 = v.get("a").unwrap().as_array().unwrap();
    println!("get().as_array a: len={} iter={:?}", arr2.len(), arr2.iter().map(|x| x.as_u64()).collect::<Vec<_>>());
    let o = v.get_object("o").unwrap();
    println!("get_object o: len={} keys={:?} y={:?}", o.len(), o.keys().collect::<Vec<_>>(), o.get("y").map(|x|x.as_u64()));
    let ta = v.try_get_array("a").unwrap().unwrap();
    println!("try_get_array a iter={:?}", ta.iter().map(|x| x.as_u64()).collect::<Vec<_>>());
    let to = v.try_get_object("o").unwrap().unwrap();
    println!("try_get_object keys={:?}", to.keys().collect::<Vec<_>>());
    let r = std::panic::catch_unwind(|| {
        let mut b = br#"{"n":[[1,2]]}"#.to_vec();
        let t = simd_json::to_tape(&mut b).unwrap();
        let v = t.as_value();
        let n = v.get_array("n").unwrap();
        n.iter().count()
    });
    println!("get_array nested iter: {:?}", r.map_err(|_| "PANIC"));
    let r = std::panic::catch_unwind(|| {
        let mut b = br#"{"m":{"k":{"z":1}}}"#.to_vec();
        let t = simd_json::to_tape(&mut b).unwrap();
        let v = t.as_value();
        let n = v.get_object("m").unwrap();
        n.iter().count()
    });
    println!("get_object nested iter: {:?}", r.map_err(|_| "PANIC"));
    // lazy
    let mut b = br#"{"a":[1,2,3],"o":{"x":1,"y":2}}"#.to_vec();
    let t = simd_json::to_tape(&mut b).unwrap();
    let lz = simd_json::value::lazy::Value::from_tape(t.as_value());
    let la = lz.get_array("a").unwrap();
    println!("lazy get_array len={} n={}", la.len(), la.iter().count());
    // tape encode
    println!("tape encode {:?}", t.as_value().encode());
    println!("tape encode_pp {:?}", t.as_value().encode_pp());
    // key with quote through all encoders
    let mut b = u(r#"{"a%"b%n":"x%"y","l":["%u0001"]}"#).into_bytes();
    let ov = simd_json::to_owned_value(&mut b.clone()).unwrap();
    let bv = simd_json::to_borrowed_value(&mut b).unwrap();
    println!("owned encode {} pp {:?}", ov.encode(), ov.encode_pp());
    println!("borrowed encode {}", bv.encode());
    let mut b = u(r#"{"a%"b%n":"x%"y","l":["%u0001"]}"#).into_bytes();
    let t = simd_json::to_tape(&mut b).unwrap();
    println!("tape encode {}", t.as_value().encode());
    let nan = OwnedValue::from(f64::NAN);
    println!("value encode NaN {}", nan.encode());
    println!("value encode Inf {}", OwnedValue::from(f64::INFINITY).encode());
    println!("value encode -Inf {}", OwnedValue::from(f64::NEG_INFINITY).encode());
    println!("serde to_string value NaN {:?}", simd_json::to_string(&nan).map_err(|e|e.to_string()));
    println!("to_string value NaN via serde_json {:?}", serde_json::to_string(&nan).map_err(|e|e.to_string()));
    // bytes in value
    println!("OwnedValue from u128? as_u128={:?}", OwnedValue::from(5u64).as_u128());
    // try-into serde_json of u64 > i64
    let big: serde_json::Value = serde_json::from_str("18446744073709551615").unwrap();
    let ov: OwnedValue = big.try_into().unwrap();
    println!("from sj big {:?}", ov);
    // deserialize OwnedValue via serde from tape
    let mut b = br#"{"a":[1,{"b":null}],"s":"x","f":1.5,"n":-3}"#.to_vec();
    let v: OwnedValue = simd_json::from_slice(&mut b).unwrap();
    println!("serde OwnedValue {:?}", v);
    let mut b = br#"{"a":[1,{"b":null}],"s":"x","f":1.5,"n":-3}"#.to_vec();
    let v: BorrowedValue = simd_json::from_slice(&mut b).unwrap();
    println!("serde BorrowedValue {:?}", v);
    // deserialize from value
    #[derive(Debug, Deserialize)] struct A { a: Vec<u8>, b: Option<String>, c: (u8,u8) }
    let mut b = br#"{"a":[1,2],"b":null,"c":[1,2]}"#.to_vec();
    let ov = simd_json::to_owned_value(&mut b).unwrap();
    println!("from OwnedValue: {:?}", A::deserialize(ov).map_err(|e|e.to_string()));
    let mut b = br#"{"a":[1,2],"b":null,"c":[1,2,3]}"#.to_vec();
    let ov = simd_json::to_owned_value(&mut b).unwrap();
    println!("from OwnedValue extra: {:?}", A::deserialize(ov).map_err(|e|e.to_string()));
    // tape with Tape::deserialize of empty
    let t = Tape::null();
    println!("Tape::null deserialize {:?}", t.deserialize::<Option<u8>>().map_err(|e|e.to_string()));
    let _ = (Node::Static(StaticNode::Null),);
    // fill_tape reuse
    let mut buffers = simd_json::Buffers::default();
    let mut tape = Tape::null();
    let mut s = br#"{"a":1}"#.to_vec();
    simd_json::fill_tape(&mut s, &mut buffers, &mut tape).unwrap();
    println!("fill {:?}", tape.0.len());
    // Buffers with small
    for n in [0usize, 1, 31, 32, 33, 63, 64, 65, 127, 128, 129, 1000] {
        let mut buffers = simd_json::Buffers::new(0);
        let st = format!("\"{}\\n\"", "a".repeat(n));
        let mut sb = st.clone().into_bytes();
        let v = simd_json::to_owned_value_with_buffers(&mut sb, &mut buffers).unwrap();
        assert_eq!(v.as_str().unwrap().len(), n + 1);
    }
    println!("buffers ok");
}
