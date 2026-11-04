use serde::Deserialize;
use std::collections::BTreeMap;

#[test]
fn integer_keys_roundtrip_through_dom() {
    let value = BTreeMap::from([(42_u32, "answer".to_string())]);
    let owned = simd_json::serde::to_owned_value(&value).unwrap();
    let borrowed = simd_json::serde::to_borrowed_value(&value).unwrap();
    let ref_owned = simd_json::serde::from_refowned_value::<BTreeMap<u32, String>>(&owned);
    let ref_borrowed = simd_json::serde::from_refborrowed_value::<BTreeMap<u32, String>>(&borrowed);
    let moved_owned = simd_json::serde::from_owned_value::<BTreeMap<u32, String>>(owned);
    let moved_borrowed = simd_json::serde::from_borrowed_value::<BTreeMap<u32, String>>(borrowed);
    println!(
        "ref_owned={ref_owned:?}, ref_borrowed={ref_borrowed:?}, moved_owned={moved_owned:?}, moved_borrowed={moved_borrowed:?}"
    );
    assert_eq!(moved_owned.unwrap(), value);
    assert_eq!(moved_borrowed.unwrap(), value);
}

#[test]
fn borrowed_dom_accepts_owned_strings_from_serde() {
    let source = serde_json::json!({"a": "b"});
    let value = simd_json::BorrowedValue::deserialize(source);
    println!("value={value:?}");
    assert!(value.is_ok());
}

#[test]
fn borrowed_dom_accepts_escaped_keys_from_serde_json() {
    let input = r#"{"a\"b":1}"#;
    let value = serde_json::from_str::<simd_json::BorrowedValue>(input);
    println!("value={value:?}");
    assert!(value.is_ok());
}

#[test]
fn exhausted_numeric_deserializer_returns_error() {
    let mut input = b"1".to_vec();
    let mut de = simd_json::Deserializer::from_slice(&mut input).unwrap();
    assert_eq!(u64::deserialize(&mut de).unwrap(), 1);
    assert!(u64::deserialize(&mut de).is_err());
}
