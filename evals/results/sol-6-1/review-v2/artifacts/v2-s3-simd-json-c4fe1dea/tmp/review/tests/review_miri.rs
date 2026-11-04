use simd_json::prelude::*;

#[test]
fn ordinary_parse_and_serialize() {
    let text = br#"{"name":"caf\u00e9","items":[1,null,true,{"x":"line\nbreak"}]}"#;
    let mut bytes = text.to_vec();
    let owned = simd_json::to_owned_value(&mut bytes).unwrap();
    let mut bytes = text.to_vec();
    let borrowed = simd_json::to_borrowed_value(&mut bytes).unwrap();
    assert_eq!(owned.encode(), borrowed.encode());
    let mut bytes = text.to_vec();
    let tape = simd_json::to_tape(&mut bytes).unwrap();
    assert_eq!(tape.as_value().encode(), owned.encode());
}

#[test]
fn exhausted_deserializer_returns_error() {
    use serde::Deserialize;
    let mut bytes = b"1".to_vec();
    let mut de = simd_json::Deserializer::from_slice(&mut bytes).unwrap();
    assert_eq!(u64::deserialize(&mut de).unwrap(), 1);
    assert!(u64::deserialize(&mut de).is_err());
}
