use simd_json::prelude::*;
fn main() {
    let which = std::env::args().nth(1).unwrap();
    let n: usize = std::env::args().nth(2).unwrap().parse().unwrap();
    let mut s = Vec::with_capacity(2*n);
    match which.as_str() {
        "owned" | "borrowed" | "serde_owned" | "tape" | "drop" | "ignored" => {
            s.extend(std::iter::repeat(b'[').take(n));
            s.extend(std::iter::repeat(b']').take(n));
        }
        _ => panic!(),
    }
    match which.as_str() {
        "owned" => { let v = simd_json::to_owned_value(&mut s).unwrap(); println!("ok {}", v.is_array()); std::mem::forget(v); }
        "borrowed" => { let v = simd_json::to_borrowed_value(&mut s).unwrap(); println!("ok {}", v.is_array()); std::mem::forget(v);}
        "serde_owned" => { let v: simd_json::OwnedValue = simd_json::from_slice(&mut s).unwrap(); println!("ok {}", v.is_array()); std::mem::forget(v);}
        "ignored" => { let v: serde::de::IgnoredAny = simd_json::from_slice(&mut s).unwrap(); println!("ok {:?}", v);}
        "tape" => { let t = simd_json::to_tape(&mut s).unwrap(); println!("tape ok {}", t.0.len()); println!("enc {}", t.as_value().encode().len()); }
        "drop" => { let mut v = simd_json::OwnedValue::array(); for _ in 0..n { let mut o = simd_json::OwnedValue::array(); o.push(v).unwrap(); v = o; } println!("built"); drop(v); println!("dropped"); }
        _ => {}
    }
}
