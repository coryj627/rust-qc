use simd_json::{prelude::*, to_owned_value, to_tape};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[test]
fn tape_nested_array_access() {
    let mut s = br#"{"a":[1,2]}"#.to_vec();
    let t = to_tape(&mut s).unwrap();
    let v = t.as_value();
    let a = v.get_array("a").unwrap();
    assert_eq!(a.iter().map(|x| x.as_u64()).collect::<Vec<_>>(), vec![Some(1), Some(2)]);
}
#[test]
fn tape_empty_array_access() {
    let mut s = br#"{"a":[]}"#.to_vec();
    let t = to_tape(&mut s).unwrap();
    assert_eq!(t.as_value().try_get_array("a").unwrap().unwrap().len(), 0);
}
#[test]
fn tape_nested_object_access() {
    let mut s = br#"{"a":{"b":1}}"#.to_vec();
    let t = to_tape(&mut s).unwrap();
    assert_eq!(t.as_value().try_get_object("a").unwrap().unwrap().get("b").unwrap().as_u64(), Some(1));
}
#[test]
fn tuple_extra_element_rejected() {
    let s = b"[[1,2],3]";
    assert!(serde_json::from_slice::<((u8,),u8)>(s).is_err());
    let got = simd_json::from_slice::<((u8,),u8)>(&mut s.to_vec());
    assert!(got.is_err(), "unexpected result: {got:?}");
}
#[test]
fn long_float_position_independent() {
    let number = "12345678901234567890.0";
    let expected = number.parse::<f64>().unwrap();
    for pad in 0..80 {
        let mut s = format!("{}{number}", " ".repeat(pad)).into_bytes();
        let v = to_owned_value(&mut s).unwrap();
        assert_eq!(v.as_f64().unwrap().to_bits(), expected.to_bits(), "padding {pad}: {v:?} vs {expected}");
    }
}
#[test]
fn invalid_long_integer_rejected() {
    for s in ["1234567890123456789x", "[1234567890123456789x]", "[1.234567890123456789012345x]"] {
        assert!(serde_json::from_str::<serde_json::Value>(s).is_err());
        assert!(to_owned_value(&mut s.as_bytes().to_vec()).is_err(), "accepted {s}");
    }
}
#[test]
fn escaped_keys_roundtrip() {
    let mut input = br#"{"a\"b\\c\nd":1}"#.to_vec();
    let v = to_owned_value(&mut input).unwrap();
    let encoded = v.encode();
    assert_eq!(serde_json::from_str::<serde_json::Value>(&encoded).unwrap(), serde_json::json!({"a\"b\\c\nd":1}));
}
#[test]
fn chars_roundtrip() {
    for c in ['"', '\\', '\n', '\0', 'a', 'é'] {
        let encoded = simd_json::to_string(&c).unwrap();
        assert_eq!(serde_json::from_str::<char>(&encoded).unwrap(), c);
    }
}
#[derive(Serialize)]
struct EscapedField { #[serde(rename="a\"b")] x: u8 }
#[test]
fn renamed_fields_escape() {
    let encoded = simd_json::to_string(&EscapedField { x: 1 }).unwrap();
    assert_eq!(serde_json::from_str::<serde_json::Value>(&encoded).unwrap(), serde_json::json!({"a\"b":1}));
}
struct UnknownLen(Vec<u8>);
impl Serialize for UnknownLen {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(None)?;
        for x in &self.0 { seq.serialize_element(x)?; }
        seq.end()
    }
}
#[test]
fn unknown_len_empty_seq() {
    let got = simd_json::to_string(&UnknownLen(vec![])).unwrap();
    assert_eq!(got, "[]");
}
#[test]
fn owned_value_numeric_map_keys() {
    let mut s = br#"{"1":2}"#.to_vec();
    let v = to_owned_value(&mut s).unwrap();
    let by_ref = BTreeMap::<u32,u32>::deserialize(&v).unwrap();
    let by_value = BTreeMap::<u32,u32>::deserialize(v).unwrap();
    assert_eq!(by_ref, by_value);
}
#[cfg(feature="128bit")]
#[test]
fn tape_128bit_access() {
    let n = u128::from(u64::MAX) + 1;
    let mut s = n.to_string().into_bytes();
    let t = to_tape(&mut s).unwrap();
    assert_eq!(t.as_value().as_u128(), Some(n));
    assert_eq!(t.as_value().cast_f64(), Some(n as f64));
}
#[cfg(feature="128bit")]
#[test]
fn large_integer_into_float() {
    let mut s = b"18446744073709551616".to_vec();
    assert_eq!(simd_json::from_slice::<f64>(&mut s).unwrap(), 18446744073709551616f64);
}
#[test]
fn deserializer_eof_returns_error() {
    let mut input = b"1".to_vec();
    let mut de = simd_json::Deserializer::from_slice(&mut input).unwrap();
    assert_eq!(u64::deserialize(&mut de).unwrap(), 1);
    assert!(u64::deserialize(&mut de).is_err());
}
#[test]
fn lazy_equality_after_noop_mutation() {
    let mut input = b"[1,2]".to_vec();
    let tape = to_tape(&mut input).unwrap();
    let mut v = simd_json::value::lazy::Value::from_tape(tape.as_value());
    let original = v.clone();
    v.as_array_mut().unwrap();
    assert_eq!(v, original);
}
#[test]
fn empty_tuple_variant_roundtrip() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    enum E { Empty() }
    let encoded = simd_json::to_string(&E::Empty()).unwrap();
    assert_eq!(encoded, serde_json::to_string(&E::Empty()).unwrap());
    assert_eq!(simd_json::from_slice::<E>(&mut encoded.into_bytes()).unwrap(), E::Empty());
}
#[test]
fn unknown_len_empty_seq_pretty() {
    let got = simd_json::to_string_pretty(&UnknownLen(vec![])).unwrap();
    assert_eq!(got, "[]");
}
#[test]
fn tape_integer_cast() {
    let mut input = b"1".to_vec();
    let tape = to_tape(&mut input).unwrap();
    assert_eq!(tape.as_value().cast_f64(), Some(1.0));
}
#[test]
fn invalid_numbers_and_offsets() {
    for num in ["1234567890123456789x", "1.234567890123456789012345x", "1.5e4294967296", "1e4294967296", "1e-4294967296", "1e4294967297"] {
        let got = to_owned_value(&mut num.as_bytes().to_vec());
        println!("{num} -> {got:?}");
        if num.contains('x') { assert!(got.is_err(), "accepted {num}"); }
        else { assert_eq!(got.ok().and_then(|v|v.as_f64()).map(f64::to_bits), num.parse::<f64>().ok().filter(|x| x.is_finite()).map(f64::to_bits), "{num}"); }
    }
}
#[test]
fn exponent_range_does_not_wrap() {
    let mut mismatches = Vec::new();
    for num in ["1e4294967296", "1e-4294967296", "1e4294967297", "1.5e4294967296"] {
        let got = to_owned_value(&mut num.as_bytes().to_vec()).ok().and_then(|v|v.as_f64());
        let expected = num.parse::<f64>().ok().filter(|x| x.is_finite());
        println!("{num}: actual={got:?}, expected={expected:?}");
        if got.map(f64::to_bits) != expected.map(f64::to_bits) { mismatches.push(num); }
    }
    assert!(mismatches.is_empty(), "{mismatches:?}");
}
#[test]
fn invalid_long_numbers_in_containers() {
    let mut accepted = Vec::new();
    for s in ["1234567890123456789x", "[1234567890123456789x]", "[1.234567890123456789012345x]", "{\"n\":1.234567890123456789012345x}", "[123x]"] {
        let got = to_owned_value(&mut s.as_bytes().to_vec());
        println!("{s} -> {got:?}");
        if got.is_ok() { accepted.push(s); }
    }
    assert!(accepted.is_empty(), "{accepted:?}");
}
#[test]
fn borrowed_value_numeric_map_keys() {
    let mut s = br#"{"1":2}"#.to_vec();
    let v = simd_json::to_borrowed_value(&mut s).unwrap();
    let by_ref = BTreeMap::<u32,u32>::deserialize(&v).unwrap();
    let by_value = BTreeMap::<u32,u32>::deserialize(v).unwrap();
    assert_eq!(by_ref, by_value);
}
#[cfg(feature="128bit")]
#[test]
fn tape_signed_128bit_access() {
    let n = i128::from(i64::MIN) - 1;
    let mut s = n.to_string().into_bytes();
    let t = to_tape(&mut s).unwrap();
    assert_eq!(t.as_value().as_i128(), Some(n));
}
#[test]
fn surrogate_pair_requires_low_surrogate() {
    let mut accepted = Vec::new();
    for s in [r#""\ud800\ue000""#, r#""\udbff\uffff""#, r#""\ud800\udfff""#] {
        let expected = serde_json::from_str::<String>(s);
        let got = simd_json::from_slice::<String>(&mut s.as_bytes().to_vec());
        println!("{s} -> actual={got:?}, reference={expected:?}");
        if got.is_ok() != expected.is_ok() { accepted.push(s); }
    }
    assert!(accepted.is_empty(), "{accepted:?}");
}
#[test]
fn valid_string_boundary_matrix() {
    for len in [0,1,15,16,17,31,32,33,63,64,65,127,128,129] {
        for suffix in ["", "\"", "\\", "\0\n\r\t", "é中🦀", "\u{10ffff}"] {
            let expected = format!("{}{suffix}", "a".repeat(len));
            let json = serde_json::to_vec(&expected).unwrap();
            for pad in [0,1,15,31,32,63,64] {
                let mut input = vec![b' ';pad];
                input.extend_from_slice(&json);
                let got = simd_json::from_slice::<String>(&mut input).unwrap();
                assert_eq!(got, expected, "len={len}, pad={pad}");
            }
        }
    }
}
