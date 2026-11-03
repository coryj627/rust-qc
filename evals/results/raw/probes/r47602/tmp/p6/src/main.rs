use simd_json::prelude::*;
fn own(s: &str) -> String {
    let mut b = s.as_bytes().to_vec();
    match simd_json::to_owned_value(&mut b) {
        Ok(v) => format!("OK {:?}", v),
        Err(e) => format!("ERR {}", e),
    }
}
fn main() {
    let cases = [
        "18446744073709551615","18446744073709551616","28446744073709551616","99999999999999999999","123456789012345678901234567890",
        "-9223372036854775808","-9223372036854775809","-18446744073709551616","-99999999999999999999",
        "340282366920938463463374607431768211455","340282366920938463463374607431768211456","-170141183460469231731687303715884105728","-170141183460469231731687303715884105729",
        "12345678901234567890x","123456789012345678901234x","[99999999999999999999999x]",
        "1.5","0.1","0.3","1e5","1E5","1.7976931348623157e308","1.7976931348623159e308","1e308","2e308","1e309","-1e309","1e-323","1e-324","4.9e-324","2.2250738585072014e-308",
        "1e400","1e-400","1.5e-400","1e1000","1e4294967296","1e-4294967296","1e65536","1e32768","1e65535","1e99999","1e-65536","0e99999","0.0e400","0.5e-400",
        "123456789012345678901234567890.5","1234567890123456789012345.5e10","0.000000000000000000000000001","0.1234567890123456789012345678","1.00000000000000000000000000x","12345678901234567890.5x",
        "9007199254740993.0","9007199254740993","0.30000000000000004","123456789.12345678","1.2e3","-0.0","-0","0e0","1e22","1e23",
        "1.0e","1.e5",".5","+1","00","-","-a","1a","1.5a","1e5a","0x10","1.0.0","--1",
    ];
    for c in cases { println!("{c:<45} {}", own(c)); }
    let pad = " ".repeat(60);
    println!("pad long frac {}", own(&format!("{pad}1.2345678901234567890123")));
    let mut b = br#"170141183460469231731687303715884105727"#.to_vec();
    println!("f64 from bigint {:?}", simd_json::from_slice::<f64>(&mut b).map_err(|e| e.to_string()));
    let mut b = br#"[1267650600228229401496703205376]"#.to_vec();
    let t = simd_json::to_tape(&mut b).unwrap();
    let tv = t.as_value();
    let e = tv.get_idx(0).unwrap();
    println!("tape as_u128={:?} as_i128={:?} cast_f64={:?} value_type={:?}", e.as_u128(), e.as_i128(), e.cast_f64(), e.value_type());
    let mut b = br#"[1267650600228229401496703205376]"#.to_vec();
    let o = simd_json::to_owned_value(&mut b).unwrap();
    let e = o.get_idx(0).unwrap();
    println!("owned as_u128={:?} as_i128={:?} cast_f64={:?} value_type={:?}", e.as_u128(), e.as_i128(), e.cast_f64(), e.value_type());
    let mut b = br#"{"a":1,"a":2}"#.to_vec();
    let o = simd_json::to_owned_value(&mut b).unwrap();
    println!("dup {:?}", o.encode());
    let mut b = br#"{"a":1,"a":2,"b":3}"#.to_vec();
    let t = simd_json::to_borrowed_value(&mut b).unwrap();
    println!("dup {:?}", t.encode());
    let mut b = br#"{"a":"x\ny"}"#.to_vec();
    let t = simd_json::to_borrowed_value(&mut b).unwrap().into_static();
    println!("static {:?}", t.encode());
}
