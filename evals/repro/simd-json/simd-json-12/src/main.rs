// A valid decimal must parse to its value, wherever it sits in the input (RFC 8259 sec. 6).
use simd_json::prelude::*;
fn main() {
    let json = br#"{"asks":[{"price":3077990.0,"amount":0.111111},{"price":3077999.0000000000000000,"amount":0.5}]}"#;
    let mut d = json.to_vec();
    let v: serde_json::Value = simd_json::from_slice(&mut d).unwrap();
    let got = v["asks"][1]["price"].as_f64().unwrap();
    let want: f64 = "3077999.0000000000000000".parse().unwrap();
    println!("simd-json from_slice price={got}; std f64 parse={want}");
    let mut d = json.to_vec();
    let o = simd_json::to_owned_value(&mut d).unwrap();
    let got2 = o.get("asks").unwrap().get_idx(1).unwrap().get("price").unwrap().as_f64().unwrap();
    println!("simd-json to_owned_value price={got2}");
    let mut wrong_offsets = vec![];
    for pad in 0..64 {
        let j = format!(r#"{{"pad":"{}","price":3077999.0000000000000000,"amount":0.5}}"#, "x".repeat(pad));
        let mut d = j.into_bytes();
        let v: serde_json::Value = simd_json::from_slice(&mut d).unwrap();
        if v["price"].as_f64() != Some(want) { wrong_offsets.push(pad); }
    }
    println!("pad values giving a wrong number: {wrong_offsets:?}");
    std::process::exit(if got != want { 42 } else { 0 });
}
