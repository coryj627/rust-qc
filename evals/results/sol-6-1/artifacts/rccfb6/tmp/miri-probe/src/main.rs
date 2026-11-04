use simd_json::prelude::*;
use serde::Deserialize;
fn main() {
    match std::env::args().nth(1).as_deref().unwrap_or("basic") {
        "basic" => {
            for text in [r#""\ud800\ue000""#, r#""\ud800\uffff""#, r#""\ud800a""#, r#""\ud800\u0000""#] {
                let mut input = text.as_bytes().to_vec();
                println!("unicode {text}: simd={:?}, serde={:?}", simd_json::from_slice::<String>(&mut input), serde_json::from_str::<String>(text));
            }
            for text in ["[1,2]", "[[1,2],3]", "[[1,2],3,4]"] {
                let mut input = text.as_bytes().to_vec();
                if text.starts_with("[[") {
                    println!("tuple {text}: simd={:?}, serde={:?}", simd_json::from_slice::<((u8,),u8)>(&mut input), serde_json::from_str::<((u8,),u8)>(text));
                } else {
                    println!("tuple {text}: simd={:?}, serde={:?}", simd_json::from_slice::<(u8,)>(&mut input), serde_json::from_str::<(u8,)>(text));
                }
            }
        },
        "empty-tape" => { println!("{:?}", simd_json::Tape(vec![]).deserialize::<u64>()); },
        "repeat" => {
            let mut input = b"1".to_vec();
            let mut de = simd_json::Deserializer::from_slice(&mut input).unwrap();
            println!("{:?}", u64::deserialize(&mut de));
            println!("{:?}", u64::deserialize(&mut de));
        },
        "strings" => {
            let mut input = br#"["a","b\nc"]"#.to_vec();
            let tape = simd_json::to_tape(&mut input).unwrap();
            println!("{}", tape.as_value().encode());
        },
        "number-junk" => {
            for text in ["1000000000000000000x", "[1000000000000000000x]", "-100000000000000000x", "[1.234567890123456789012345x]", "0.000000000000000000000000000000000000000000000000012345678901234567890"] {
                let mut input = text.as_bytes().to_vec();
                println!("{text}: simd={:?}, serde={:?}", simd_json::to_owned_value(&mut input), serde_json::from_str::<serde_json::Value>(text));
            }
        },
        _ => panic!("unknown")
    }
}
