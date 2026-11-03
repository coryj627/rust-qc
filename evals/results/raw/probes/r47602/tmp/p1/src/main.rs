use serde::{Deserialize, Serialize, Serializer};
use simd_json::prelude::*;
use simd_json::{Deserializer, OwnedValue, BorrowedValue};

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
fn show(label: &str, s: &str) {
    let shown: String = s.chars().take(70).collect();
    println!("[{label}] input={shown:?}\n   simd : {}\n   serde: {}", own(s), sj(s));
}
struct EmptyIter;
impl Serialize for EmptyIter {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let v = vec![1u8, 2];
        s.collect_seq(v.iter().filter(|_| false))
    }
}
#[derive(Serialize)]
enum E { Tup(u8, u8), Sv { a: u8, b: u8 }, Empty(), Nt(u8), Es {} }
#[derive(Serialize)]
struct S2 { a: u8, b: u8 }

fn main() {
    // numbers
    show("exp wrap 2^32", "1e4294967296");
    show("exp wrap 2^32+1", "1e4294967297");
    show("exp wrap neg", "1e-4294967296");
    show("exp big", "1.5e4294967300");
    let pad60 = " ".repeat(60);
    show("long frac at 60", &format!("{pad60}1.2345678901234567890123"));
    for p in [50usize, 55, 60, 65, 70] {
        let pad = " ".repeat(p);
        show("long frac pad", &format!("{pad}12345678901234567890123.5"));
    }
    show("long frac nopad", "12345678901234567890123.5");
    show("trail garbage slowpath float", "1.00000000000000000000000x");
    show("trail garbage slowpath float2", "[1.00000000000000000000000x]");
    show("trail garbage bigint", "12345678901234567890x");
    show("trail garbage bigint2", "[123456789012345678901x]");
    show("trail garbage 18", "[1234567890123456789x]");
    show("neg zero", "-0");
    show("num NUL", "123\u{0}abc");
    show("true NUL", "true\u{0}abc");
    show("[1 NUL]", "[1\u{0},2]");
    show("-big", "-9223372036854775809");
    show("big u64max", "18446744073709551615");
    show("big u64max+1", "18446744073709551616");
    show("1e400", "1e400");
    show("big -1e400", "-1e400");
    show("0.0e999", "0.0e999");
    show("0e99999999999999", "0e99999999999999");
    show("1e-400", "1e-400");
    show("lead0", "01");
    show("-01", "-01");
    // strings
    show("lone high", r#""\uD83D""#);
    show("lone high+text", r#""\uD83Dabc""#);
    show("hi + non-low E000", r#""\uD800""#);
    show("hi + FFFF", r#""\uD800￿""#);
    show("hi + ascii", r#""\uD800A""#);
    show("lone low", r#""\uDC00""#);
    show("bad hex", r#""\u12G4""#);
    show("hi+bad hex", r#""\uD800\u12G4""#);
    show("hi+esc", r#""\uD800\n""#);
    show("trunc u", r#""\u12""#);
    show("ctrl in str", "\"a\u{1}b\"");
    show("del", "\"a\u{7f}b\"");
    show("bad esc", r#""\x""#);
    show("dup", r#"{"a":1,"a":2}"#);
    // structure
    show("trailing comma", "[1,]");
    show("obj trailing", "{\"a\":1,}");
    show("trail", "[1] x");
    show("empty", "");
    show("ws", "   ");
    show("tr", "[1,");
    show("two", "1 2");
    show("[]]", "[]]");
    show("{}x", "{} {}");
    show("ws ff", "\u{c}1");
    show("nbsp", "1\u{a0}");
    show("bom", "\u{feff}1");

    // duplicates
    let mut b = br#"{"a":1,"a":2}"#.to_vec();
    let v = simd_json::to_owned_value(&mut b).unwrap();
    println!("dup: get a={:?} len={} encode={}", v.get("a"), v.as_object().unwrap().len(), v.encode());
    let mut s = String::from("{");
    for i in 0..100 { if i>0 {s.push(',');} s.push_str(&format!("\"k\":{i}")); }
    s.push('}');
    let mut b = s.into_bytes();
    let v = simd_json::to_owned_value(&mut b).unwrap();
    println!("dup100: get k={:?} len={} ", v.get("k"), v.as_object().unwrap().len());
    let mut o = v.clone();
    println!("remove k = {:?}, then get = {:?}, len={}", o.remove("k"), o.get("k"), o.as_object().unwrap().len());
    let sv: serde_json::Result<std::collections::BTreeMap<String,u8>> = serde_json::from_str(r#"{"a":1,"a":2}"#);
    println!("serde map last-wins: {:?}", sv.map(|m| m["a"]).ok());
    let mut b = br#"{"a":1,"a":2}"#.to_vec();
    let m: std::collections::BTreeMap<String,u8> = simd_json::from_slice(&mut b).unwrap();
    println!("simd map: {:?}", m);

    // serializer
    println!("collect_seq empty unknown-len: {:?} serde={:?}", simd_json::to_string(&EmptyIter).unwrap(), serde_json::to_string(&EmptyIter).unwrap());
    println!("pretty collect_seq: {:?} serde={:?}", simd_json::to_string_pretty(&EmptyIter).unwrap(), serde_json::to_string_pretty(&EmptyIter).unwrap());
    println!("NaN: {:?} serde={:?}", simd_json::to_string(&f64::NAN), serde_json::to_string(&f64::NAN).ok());
    println!("Inf: {:?} serde={:?}", simd_json::to_string(&f64::INFINITY), serde_json::to_string(&f64::INFINITY).ok());
    println!("f32 0.1: {:?} serde={:?}", simd_json::to_string(&0.1f32), serde_json::to_string(&0.1f32).ok());
    println!("pretty tup: {:?}", simd_json::to_string_pretty(&E::Tup(1,2)));
    println!("serde  tup: {:?}", serde_json::to_string_pretty(&E::Tup(1,2)));
    println!("pretty empty tup: {:?}", simd_json::to_string_pretty(&E::Empty()));
    println!("serde  empty tup: {:?}", serde_json::to_string_pretty(&E::Empty()));
    println!("compact empty tup: {:?}", simd_json::to_string(&E::Empty()));
    println!("pretty sv: {:?}", simd_json::to_string_pretty(&E::Sv{a:1,b:2}));
    println!("serde  sv: {:?}", serde_json::to_string_pretty(&E::Sv{a:1,b:2}));
    println!("pretty es: {:?}", simd_json::to_string_pretty(&E::Es{}));
    println!("compact es: {:?}", simd_json::to_string(&E::Es{}));
    println!("pretty nt: {:?}", simd_json::to_string_pretty(&E::Nt(1)));
    println!("pretty s2: {:?}", simd_json::to_string_pretty(&S2{a:1,b:2}));
    println!("serde  s2: {:?}", serde_json::to_string_pretty(&S2{a:1,b:2}));
    println!("pretty nested: {:?}", simd_json::to_string_pretty(&vec![vec![S2{a:1,b:2}]]));
    println!("pretty empty vec,map: {:?} {:?}", simd_json::to_string_pretty(&Vec::<u8>::new()), simd_json::to_string_pretty(&std::collections::BTreeMap::<String,u8>::new()));
    println!("char quote: {:?} serde {:?}", simd_json::to_string(&'"'), serde_json::to_string(&'"').ok());
    println!("char nl: {:?} serde {:?}", simd_json::to_string(&'\n'), serde_json::to_string(&'\n').ok());
    #[derive(Serialize)] struct R { #[serde(rename="a\"b\n")] x:u8 }
    println!("rename: {:?} serde {:?}", simd_json::to_string(&R{x:1}), serde_json::to_string(&R{x:1}).ok());
    println!("str ctrl: {:?} serde {:?}", simd_json::to_string("a\u{1}\u{7f}\"\\/\u{8}\u{c}"), serde_json::to_string("a\u{1}\u{7f}\"\\/\u{8}\u{c}").ok());
    let ls = "a".repeat(40) + "\"" + &"b".repeat(40) + "\n\u{1}é" ;
    println!("long str eq: {:?}", simd_json::to_string(&ls).ok()==serde_json::to_string(&ls).ok());
    println!("int key: {:?}", simd_json::to_string(&std::collections::BTreeMap::from([(1u8,2u8)])));
    println!("bool key: {:?} serde {:?}", simd_json::to_string(&std::collections::BTreeMap::from([(true,2u8)])).map_err(|e|e.to_string()), serde_json::to_string(&std::collections::BTreeMap::from([(true,2u8)])).map_err(|e|e.to_string()));

    // serde-de
    let mut b = b"[1,2,3]".to_vec();
    println!("(u8,u8) from [1,2,3]: {:?}", simd_json::from_slice::<(u8,u8)>(&mut b).map_err(|e|e.to_string()));
    println!("   serde: {:?}", serde_json::from_str::<(u8,u8)>("[1,2,3]").map_err(|e|e.to_string()));
    let mut b = b"[[1,2],3]".to_vec();
    println!("((u8,),u8) from [[1,2],3]: {:?}", simd_json::from_slice::<((u8,),u8)>(&mut b).map_err(|e|e.to_string()));
    let mut b = b"[[1,2],[3]]".to_vec();
    println!("Vec<[u8;1]> from [[1,2],[3]]: {:?}", simd_json::from_slice::<Vec<[u8;1]>>(&mut b).map_err(|e|e.to_string()));
    #[derive(Debug, Deserialize)] struct T2(u8,u8);
    #[derive(Debug, Deserialize)] struct W { a: T2, b: u8 }
    let mut b = br#"{"a":[1,2,7],"b":9}"#.to_vec();
    println!("W: {:?}", simd_json::from_slice::<W>(&mut b).map_err(|e|e.to_string()));
    #[derive(Debug, Deserialize)] struct P { x: u8, y: u8 }
    let mut b = br#"[[1,2,3],9,8]"#.to_vec();
    println!("Vec<P>..: {:?}", simd_json::from_slice::<Vec<P>>(&mut b).map_err(|e|e.to_string()));
    let mut b = br#"[1,2]"#.to_vec();
    println!("(u8,u8,u8): {:?}", simd_json::from_slice::<(u8,u8,u8)>(&mut b).map_err(|e|e.to_string()));
    // unknown enum shapes
    #[derive(Debug, Deserialize)] enum En { A, B(u8), C{a:u8}, D(u8,u8) }
    for s in [r#""A""#, r#"{"A":null}"#, r#"{"B":1}"#, r#"{"C":{"a":1}}"#, r#"{"D":[1,2]}"#, r#"{"C":[1]}"#, r#""B""#, r#"{"Z":1}"#, r#"{"B":1,"C":2}"#, r#"{"D":[1,2,3]}"#] {
        let mut b = s.as_bytes().to_vec();
        println!("enum {s}: {:?} serde {:?}", simd_json::from_slice::<En>(&mut b).map_err(|e|e.to_string()), serde_json::from_str::<En>(s).map_err(|e|e.to_string()));
    }
    // u64 big from float etc
    let mut b = b"1.5".to_vec();
    println!("u8 from 1.5: {:?}", simd_json::from_slice::<u8>(&mut b).map_err(|e|e.to_string()));
    let mut b = b"300".to_vec();
    println!("u8 from 300: {:?}", simd_json::from_slice::<u8>(&mut b).map_err(|e|e.to_string()));
    let mut b = b"-1".to_vec();
    println!("u8 from -1: {:?}", simd_json::from_slice::<u8>(&mut b).map_err(|e|e.to_string()));
    let mut b = b"18446744073709551615".to_vec();
    println!("i64 from u64max: {:?}", simd_json::from_slice::<i64>(&mut b).map_err(|e|e.to_string()));
    let mut b = b"1e400".to_vec();
    println!("f32 from 1e39 : {:?}", simd_json::from_slice::<f32>(&mut b"1e39".to_vec()).map_err(|e|e.to_string()));
    let _ = b;
    let mut b = b"5".to_vec();
    println!("str key ints: {:?}", simd_json::from_slice::<f64>(&mut b).map_err(|e|e.to_string()));
    let mut b = br#"{"1":2}"#.to_vec();
    println!("HashMap<u8,u8>: {:?}", simd_json::from_slice::<std::collections::HashMap<u8,u8>>(&mut b).map_err(|e|e.to_string()));
    let mut b = br#"{"true":2}"#.to_vec();
    println!("HashMap<bool,u8>: {:?} serde {:?}", simd_json::from_slice::<std::collections::HashMap<bool,u8>>(&mut b).map_err(|e|e.to_string()), serde_json::from_str::<std::collections::HashMap<bool,u8>>(r#"{"true":2}"#).map_err(|e|e.to_string()));
    let mut b = br#"{"1.5":2}"#.to_vec();
    println!("f64 key: {:?} serde {:?}", simd_json::from_slice::<Vec<(f64,u8)>>(&mut b).map_err(|e|e.to_string()), 0);
    let mut b = br#"{"a":1}"#.to_vec();
    println!("Option key: {:?}", simd_json::from_slice::<std::collections::BTreeMap<Option<String>,u8>>(&mut b).map_err(|e|e.to_string()));
    let mut b = br#"{"A\n":"x\ty"}"#.to_vec();
    println!("borrowed esc: {:?}", simd_json::from_slice::<std::collections::BTreeMap<&str,&str>>(&mut b).map_err(|e|e.to_string()));
    // errors PartialEq
    println!("InputTooLarge==: {}", simd_json::ErrorType::InputTooLarge == simd_json::ErrorType::InputTooLarge);
    println!("SimdUnsupported==: {}", simd_json::ErrorType::SimdUnsupported == simd_json::ErrorType::SimdUnsupported);
    println!("Unexpected==: {}", simd_json::ErrorType::Unexpected(None,None) == simd_json::ErrorType::Unexpected(None,None));
    let e = simd_json::to_owned_value(&mut b"[tru]".to_vec()).unwrap_err();
    println!("[tru] {:?} syntax={} data={}", e.error(), e.is_syntax(), e.is_data());
    let e = simd_json::to_owned_value(&mut b"[1,".to_vec()).unwrap_err();
    println!("[1, {:?} eof={} syntax={}", e.error(), e.is_eof(), e.is_syntax());
    let e = simd_json::to_owned_value(&mut b"\"abc".to_vec()).unwrap_err();
    println!("unterminated {:?} eof={} syntax={}", e.error(), e.is_eof(), e.is_syntax());
    // error index
    let e = simd_json::to_owned_value(&mut br#"     {"abc": "xyz\q"}"#.to_vec()).unwrap_err();
    println!("bad escape index={} char={:?}  (actual backslash at 16)", e.index(), e.character());
    let e = simd_json::to_owned_value(&mut br#"     {"abc": "xyz\uZZZZ"}"#.to_vec()).unwrap_err();
    println!("bad unicode index={} char={:?} (actual backslash at 16)", e.index(), e.character());
    let e = simd_json::to_owned_value(&mut b"        1e400".to_vec()).unwrap_err();
    println!("1e400 index={} char={:?}", e.index(), e.character());
    let e = simd_json::to_owned_value(&mut b"        [1.5e400]".to_vec()).unwrap_err();
    println!("[1.5e400] index={} char={:?}", e.index(), e.character());
    // tape
    let mut b = br#"[1,2.5,{"a":1}]"#.to_vec();
    let t = simd_json::to_tape(&mut b).unwrap();
    let tv = t.as_value();
    let one = tv.get_idx(0).unwrap();
    println!("tape cast_f64(1)={:?} owned={:?}", one.cast_f64(), OwnedValue::from(1u64).cast_f64());
    println!("tape as_i128(1)={:?}", one.as_i128());
    let ld = simd_json::value::lazy::Value::from_tape(tv);
    let mut ld2 = ld.clone();
    let _ = ld2.push(3);
    let _ = ld2.pop();
    println!("lazy tape==upgraded: {}", ld == ld2);
    let _: Option<BorrowedValue> = None;
    // Deep nest done separately

    // Double deserialize in safe code
    println!("deserializer double");
    let mut b = b"1".to_vec();
    let mut d = Deserializer::from_slice(&mut b).unwrap();
    println!(" first {:?}", u8::deserialize(&mut d).map_err(|e|e.to_string()));
    println!(" second {:?}", u8::deserialize(&mut d).map_err(|e|e.to_string()));
    println!(" third {:?}", String::deserialize(&mut d).map_err(|e|e.to_string()));
}
