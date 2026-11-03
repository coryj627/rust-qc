use simd_json::prelude::*;
use serde::Deserialize;
use std::collections::BTreeMap;
fn main(){
    println!("i128 key to_string {:?}", simd_json::to_string(&BTreeMap::from([(1i128,1u8)])).map_err(|e|e.to_string()));
    println!("i128 key to_value {:?}", simd_json::serde::to_owned_value(&BTreeMap::from([(1i128,1u8)])).map(|v| v.encode()).map_err(|e|e.to_string()));
    println!("u128 key to_value {:?}", simd_json::serde::to_owned_value(&BTreeMap::from([(1u128,1u8)])).map(|v| v.encode()).map_err(|e|e.to_string()));
    let v = simd_json::OwnedValue::from(1u64);
    println!("Display scalar {}", v);
    let mut b = br#"{"a":[1,"x"],"b":null}"#.to_vec();
    let ov = simd_json::to_owned_value(&mut b).unwrap();
    println!("Display object {}", ov);
    let mut pad = " ".repeat(40);
    pad.push_str("{\"k\":777253035878364813707596e287,\"x\":1}");
    let mut b=pad.clone().into_bytes();
    println!("{:?}", simd_json::to_owned_value(&mut b).map(|v| v.encode()).map_err(|e|e.to_string()));
    for offs in [0usize,40,45,48,50,55,60,65,70,80,100] {
        let d = format!("{}1.2345678901234567890123", " ".repeat(offs));
        let mut b=d.into_bytes();
        println!("offs {offs} {:?}", simd_json::to_owned_value(&mut b).map(|v| v.encode()).map_err(|e|e.to_string()));
    }
    // realistic
    let d = r#"{"id":12345,"name":"sensor-a","readings":{"lat":40.712800000000000000001}}"#;
    let mut b=d.as_bytes().to_vec();
    println!("{d}\n -> {:?}", simd_json::to_owned_value(&mut b).map(|v| v.encode()).map_err(|e|e.to_string()));
    let d = r#"{"id":12345,"name":"sensor-abcdef","reading":3.14159265358979323846264338}"#;
    let mut b=d.as_bytes().to_vec();
    println!("{d}\n -> {:?}", simd_json::to_owned_value(&mut b).map(|v| v.encode()).map_err(|e|e.to_string()));
    let _ = i32::deserialize(simd_json::OwnedValue::from(1));
}
