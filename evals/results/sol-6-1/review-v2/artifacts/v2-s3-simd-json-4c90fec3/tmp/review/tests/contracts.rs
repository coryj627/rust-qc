use simd_json::prelude::*;

#[test]
fn tuple_must_reject_extra_element() {
    let input = "[[1,2],3]";
    let expected = serde_json::from_str::<((u8,), u8)>(input);
    let actual = simd_json::from_slice::<((u8,), u8)>(&mut input.as_bytes().to_vec());
    println!("expected={expected:?}, actual={actual:?}");
    assert!(actual.is_err());
}

#[test]
fn tuple_must_not_take_next_array_element() {
    let input = "[[1],2]";
    let expected = serde_json::from_str::<((u8, u8),)>(input);
    let actual = simd_json::from_slice::<((u8, u8),)>(&mut input.as_bytes().to_vec());
    println!("expected={expected:?}, actual={actual:?}");
    assert!(actual.is_err());
}

#[test]
fn object_keys_escape_on_encode() {
    let input = r#"{"quote\"":1,"backslash\\":2,"newline\n":3}"#;
    let value = simd_json::to_owned_value(&mut input.as_bytes().to_vec()).unwrap();
    let encoded = value.encode();
    println!("encoded={encoded:?}");
    let reparsed = serde_json::from_str::<serde_json::Value>(&encoded);
    assert_eq!(
        reparsed.unwrap(),
        serde_json::from_str::<serde_json::Value>(input).unwrap()
    );
}

#[test]
fn lazy_value_semantics_survive_upgrade() {
    let mut input = br#"{"a":1}"#.to_vec();
    let tape = simd_json::to_tape(&mut input).unwrap();
    let before = simd_json::value::lazy::Value::from_tape(tape.as_value());
    let mut after = before.clone();
    assert!(after.as_object_mut().is_some());
    assert_eq!(before.clone().into_value(), after.clone().into_value());
    println!("before={before}, after={after}, equal={}", before == after);
    assert_eq!(before, after);
}
