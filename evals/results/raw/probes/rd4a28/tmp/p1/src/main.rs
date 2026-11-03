use simd_json::prelude::*;
use std::collections::HashMap;
fn own(s: &str) -> String { let mut b = s.as_bytes().to_vec(); match simd_json::to_owned_value(&mut b) { Ok(v) => format!("OK {:?}", v), Err(e) => format!("ERR {e} idx={} type={:?}", e.index(), e.error()) } }
fn main() {
    let cases: Vec<String> = vec![
      "1234567890123456789abc".into(), "[1234567890123456789abc]".into(), "[12345678901234567890123abc, 1]".into(),
      "1.2345678901234567890123abc".into(), "[1.2345678901234567890123abc]".into(), "1234567890123456789012.5xyz".into(),
      "1e4294967296".into(), "1e4294967297".into(), "1e-4294967297".into(), "1.5e4294967298".into(), "1e2147483648".into(),"0e99999999999".into(),"1e-99999999999".into(),
      r#""\ud800""#.into(), r#""\ud800abc""#.into(), r#""\ud83dxy""#.into(), r#""\ud800""#.into(), r#""\ud800￿""#.into(), r#""\udc00""#.into(), r#""\ud800A""#.into(), r#""\uZZZZ""#.into(),
      r#"{"long key padding padding": "\uZZZZ"}"#.into(), r#"["aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","\q"]"#.into(),
      "123\0".into(), "[123\0]".into(), "true\0".into(), "[true\0]".into(), "[null\0,1]".into(), "1.5\0".into(),
      "[1,]".into(), "{\"a\":1,}".into(), "[1]x".into(), "{}x".into(), "x".into(), "".into(), "   ".into(), "[".into(), "{".into(), "]".into(), "}".into(), ",".into(), ":".into(), "[1 2]".into(), "[[]".into(), "[]]".into(), "{\"a\"}".into(), "{\"a\":}".into(), "{1:2}".into(),
      "-".into(), "-a".into(), "01".into(), "-01".into(), "1.".into(), "1e".into(), "+1".into(), ".5".into(), "-0".into(), "-0.0".into(), "0".into(), "18446744073709551615".into(), "18446744073709551616".into(), "-9223372036854775808".into(), "-9223372036854775809".into(),
      "99999999999999999".into(), "999999999999999999".into(), "9999999999999999999".into(), "-999999999999999999".into(),
      "\u{feff}[1]".into(), "\"abc".into(),"\"abc\\\"".into(), "\"\t\"".into(), "\"\x7f\"".into(),
    ];
    for c in &cases { println!("{:?} -> {}", c, own(c)); }
    // float at offsets
    for off in [0usize, 10, 40, 49,50, 55, 60, 65, 70, 80, 100, 200, 1000] {
        let s = format!("{}{}", " ".repeat(off), "1.2345678901234567890123");
        println!("off {off}: {}", own(&s));
        let s = format!("{}{}", " ".repeat(off), "0.00000000000000000000012345678901234567890123");
        println!("off0 {off}: {}", own(&s));
    }
    // dup keys
    println!("dup {}", own(r#"{"a":1,"a":2}"#));
    let mut b = br#"{"a":1,"a":2}"#.to_vec(); let v = simd_json::to_owned_value(&mut b).unwrap(); println!("v[a]={} len={:?} str={}", v["a"], v.as_object().map(|o|o.len()), simd_json::to_string(&v).unwrap());
    let mut b = br#"{"a":1,"a":2}"#.to_vec(); let v = simd_json::to_borrowed_value(&mut b).unwrap(); println!("bv[a]={} len={:?}", v["a"], v.as_object().map(|o|o.len()));
    let mut b = br#"{"a":1,"a":2}"#.to_vec(); let t = simd_json::to_tape(&mut b).unwrap(); println!("tape get a={:?}", t.as_value().get("a").map(|v| v.as_u64()));
    let mut s = String::from("{"); for i in 0..40 { s += &format!("\"k{i}\":{i},"); } s += "\"k5\":999}";
    let mut b = s.clone().into_bytes(); let v = simd_json::to_owned_value(&mut b).unwrap(); println!("big v[k5]={} len={:?}", v["k5"], v.as_object().map(|o|o.len()));
    let mut b = s.clone().into_bytes(); let m: HashMap<String,u32> = simd_json::from_slice(&mut b).unwrap(); println!("serde k5={}", m["k5"]);
    // serde arity
    let mut b = b"[1,2]".to_vec(); println!("tuple1 {:?}", simd_json::from_slice::<(u8,)>(&mut b).map_err(|e|e.to_string()));
    let mut b = b"[[1,[7]],[3,4]]".to_vec(); println!("vec tuple {:?}", simd_json::from_slice::<Vec<(u8,)>>(&mut b).map_err(|e|e.to_string()));
    #[derive(Debug, serde::Deserialize)] struct S{a:u8,b:u8}
    let mut b = b"[1,2,3]".to_vec(); println!("struct arr {:?}", simd_json::from_slice::<S>(&mut b).map_err(|e|e.to_string()));
    let mut b = b"[[1,2,3],[4,[5,6],7]]".to_vec(); println!("vec struct {:?}", simd_json::from_slice::<Vec<S>>(&mut b).map_err(|e|e.to_string()));
    let mut b = br#"{"a":1,"b":2,"a":3}"#.to_vec(); println!("dup struct {:?}", simd_json::from_slice::<S>(&mut b).map_err(|e|e.to_string()));
    let mut b = br#"{"a":1,"b":2} "#.to_vec(); println!("ok struct {:?}", simd_json::from_slice::<S>(&mut b).map_err(|e|e.to_string()));
    // two deserialize
    let r = std::panic::catch_unwind(|| { use serde::Deserialize; let mut b=b"1".to_vec(); let mut d = simd_json::Deserializer::from_slice(&mut b).unwrap(); let a=u8::deserialize(&mut d); let c=u8::deserialize(&mut d); format!("{a:?} {c:?}")}); println!("twice {:?}", r.map_err(|_|"panic"));
    // keys
    let mut m = HashMap::new(); m.insert(true, 1); let s = simd_json::to_string(&m); println!("bool key {:?}", s.as_ref().map_err(|e|e.to_string())); if let Ok(s)=s { let mut b=s.into_bytes(); println!(" back {:?}", simd_json::from_slice::<HashMap<bool,i32>>(&mut b).map_err(|e|e.to_string())); }
    let mut m = HashMap::new(); m.insert(5u128, 1); let s = simd_json::to_string(&m); println!("u128 key {:?}", s.as_ref().map_err(|e|e.to_string())); if let Ok(s)=s { let mut b=s.into_bytes(); println!(" back {:?}", simd_json::from_slice::<HashMap<u128,i32>>(&mut b).map_err(|e|e.to_string())); }
    let mut m = HashMap::new(); m.insert('c', 1); let s = simd_json::to_string(&m); println!("char key {:?}", s.as_ref().map_err(|e|e.to_string())); if let Ok(s)=s { let mut b=s.into_bytes(); println!(" back {:?}", simd_json::from_slice::<HashMap<char,i32>>(&mut b).map_err(|e|e.to_string())); }
    println!("nan {:?}", simd_json::to_string(&f64::NAN).map_err(|e|e.to_string())); println!("inf {:?}", simd_json::to_string(&f32::INFINITY).map_err(|e|e.to_string()));
    println!("f32 {:?} {:?}", simd_json::to_string(&0.1f32).map_err(|e|e.to_string()), serde_json::to_string(&0.1f32).unwrap());
    println!("u128 {:?} i128 {:?}", simd_json::to_string(&u128::MAX).map_err(|e|e.to_string()), simd_json::to_string(&i128::MIN).map_err(|e|e.to_string()));
    println!("ctrl {:?}", simd_json::to_string("a\u{1}\u{7f}\"\\\n\u{2028}é😀").map_err(|e|e.to_string()));
}
