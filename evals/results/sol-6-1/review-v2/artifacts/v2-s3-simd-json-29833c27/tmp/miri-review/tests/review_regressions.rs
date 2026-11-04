use simd_json::{prelude::*, value::lazy, OwnedValue};
use serde::ser::{Serialize, Serializer, SerializeSeq};

#[test]
fn tuple_extra_elements_rejected() {
    let input = "[[1,2,3],4]";
    let reference = serde_json::from_str::<((u8,u8),u8)>(input);
    let actual = simd_json::from_slice::<((u8,u8),u8)>(&mut input.as_bytes().to_vec());
    println!("reference={reference:?}, actual={actual:?}");
    assert!(actual.is_err());
}

#[test]
fn tape_container_accessors_preserve_contents() {
    let mut input = br#"{"array":[1,2],"object":{"x":1},"empty_array":[],"empty_object":{}}"#.to_vec();
    let tape = simd_json::to_tape(&mut input).unwrap();
    let root = tape.as_value();
    for key in ["array", "empty_array"] {
        let direct = root.get_array(key).unwrap();
        let via_value = root.get(key).unwrap().as_array().unwrap();
        println!("{key}: len={}, iter count={}, generic count={}", direct.len(), direct.iter().count(), via_value.iter().count());
        assert_eq!(direct.iter().count(), via_value.iter().count(), "{key}");
    }
    for key in ["object", "empty_object"] {
        let direct = root.try_get_object(key).unwrap().unwrap();
        let via_value = root.get(key).unwrap().as_object().unwrap();
        assert_eq!(direct.iter().count(), via_value.iter().count(), "{key}");
    }
}

struct UnknownLengthEmpty;
impl Serialize for UnknownLengthEmpty {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_seq(None)?.end()
    }
}
#[test]
fn unknown_length_empty_sequence() {
    let actual = simd_json::to_string(&UnknownLengthEmpty).unwrap();
    let pretty = simd_json::to_string_pretty(&UnknownLengthEmpty).unwrap();
    let reference = serde_json::to_string(&UnknownLengthEmpty).unwrap();
    println!("compact={actual:?}, pretty={pretty:?}, reference={reference:?}");
    assert_eq!(actual, reference);
}

#[test]
fn number_exponent_boundaries() {
    for input in ["1e4294967296", "1e-4294967296", "1e4294967297", "1e-4294967297"] {
        let reference = input.parse::<f64>().unwrap();
        let actual = simd_json::to_owned_value(&mut input.as_bytes().to_vec());
        println!("{input}: std={reference:?}, simd={actual:?}");
        if reference.is_finite() {
            assert_eq!(actual.unwrap().as_f64(), Some(reference), "{input}");
        } else {
            assert!(actual.is_err(), "{input}");
        }
    }
}

#[test]
fn large_integer_trailing_letters_rejected() {
    for input in ["1234567890123456789x", "[1234567890123456789x]", "-123456789012345678x"] {
        let actual = simd_json::to_owned_value(&mut input.as_bytes().to_vec());
        println!("{input}: {actual:?}");
        assert!(actual.is_err(), "{input}");
    }
}

#[test]
fn unpaired_high_surrogate_rejected() {
    for input in [r#""\ud800""#, r#""\ud800\ud800""#, r#""\ud800\ue000""#] {
        let reference = serde_json::from_str::<String>(input);
        let actual = simd_json::from_slice::<String>(&mut input.as_bytes().to_vec());
        println!("{input}: reference={reference:?}, actual={actual:?}");
        assert!(actual.is_err(), "{input}");
    }
}

#[test]
fn nonfinite_float_serialization() {
    for number in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let actual = simd_json::to_string(&number);
        let reference = serde_json::to_string(&number);
        let dom = OwnedValue::from(number).encode();
        println!("{number:?}: actual={actual:?}, reference={reference:?}, dom={dom}");
        assert_eq!(actual.unwrap(), reference.unwrap());
    }
}

#[test]
fn lazy_value_equality_survives_upgrade() {
    let mut input = br#"{"x":1}"#.to_vec();
    let tape = simd_json::to_tape(&mut input).unwrap();
    let first = lazy::Value::from_tape(tape.as_value());
    let mut second = first.clone();
    assert_eq!(first, second);
    second.as_object_mut().unwrap();
    println!("first={first}, second={second}");
    assert_eq!(first, second);
}

#[test]
fn tape_128_scalar_access() {
    let input = "9223372036854775808";
    let mut bytes = input.as_bytes().to_vec();
    let tape = simd_json::to_tape(&mut bytes).unwrap();
    let dom = simd_json::to_owned_value(&mut input.as_bytes().to_vec()).unwrap();
    println!("tape as_i128={:?}, dom={:?}", tape.as_value().as_i128(), dom.as_i128());
    assert_eq!(tape.as_value().as_i128(), dom.as_i128());
}

#[test]
fn long_float_independent_of_offset() {
    let number = "12345678901234567890.0";
    let expected = number.parse::<f64>().unwrap();
    for offset in [0, 32, 49, 50, 51, 60, 64, 70, 100] {
        let mut input = format!("{}{number}", " ".repeat(offset)).into_bytes();
        let actual = simd_json::to_owned_value(&mut input).unwrap().as_f64().unwrap();
        println!("offset={offset}, actual={actual}, expected={expected}");
        assert_eq!(actual, expected, "offset={offset}");
    }
}

#[test]
fn consume_dom_numeric_map_keys() {
    use std::collections::BTreeMap;
    type Map = BTreeMap<u32, u32>;
    let mut input = br#"{"1":2}"#.to_vec();
    let dom = simd_json::to_owned_value(&mut input).unwrap();
    let by_ref = simd_json::serde::from_refowned_value::<Map>(&dom);
    let by_value = simd_json::serde::from_owned_value::<Map>(dom);
    println!("by ref={by_ref:?}, by value={by_value:?}");
    assert_eq!(by_value, by_ref);
}

#[test]
#[cfg(feature = "128bit")]
fn tape_large_128_scalar_access() {
    let input = "18446744073709551616";
    let mut bytes = input.as_bytes().to_vec();
    let tape = simd_json::to_tape(&mut bytes).unwrap();
    let dom = simd_json::to_owned_value(&mut input.as_bytes().to_vec()).unwrap();
    println!("tape as_u128={:?}, dom={:?}", tape.as_value().as_u128(), dom.as_u128());
    assert_eq!(tape.as_value().as_u128(), dom.as_u128());
}

#[test]
#[cfg(feature = "128bit")]
fn deserialize_large_integer_as_float() {
    let input = "18446744073709551616";
    let actual = simd_json::from_slice::<f64>(&mut input.as_bytes().to_vec());
    println!("{input} to f64 = {actual:?}");
    assert_eq!(actual.unwrap(), input.parse::<f64>().unwrap());
}

#[test]
fn smoke_valid_roundtrip() {
    let mut input = br#"{"a":[1,2,3],"s":"hello\nworld","b":true,"n":null}"#.to_vec();
    let tape = simd_json::to_tape(&mut input).unwrap();
    let output = tape.as_value().encode();
    let roundtrip: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(roundtrip["s"], "hello\nworld");
}
