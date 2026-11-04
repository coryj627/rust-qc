use serde::Deserialize;
use simd_json::prelude::*;

#[test]
fn reject_extra_tuple_elements() {
    let mut input = b"[1,2,3]".to_vec();
    let actual = simd_json::from_slice::<(u64, u64)>(&mut input);
    assert!(
        actual.is_err(),
        "extra tuple element was accepted: {actual:?}"
    );
}

#[test]
fn nested_tuple_does_not_shift_siblings() {
    let mut input = b"[[1,2,3],4]".to_vec();
    let actual = simd_json::from_slice::<((u64, u64), u64)>(&mut input);
    assert!(
        actual.is_err(),
        "extra nested element changed sibling value: {actual:?}"
    );
}

#[test]
fn reject_extra_array_struct_fields() {
    #[derive(Debug, Deserialize)]
    struct Point {
        x: u64,
        y: u64,
    }
    let mut input = b"[1,2,3]".to_vec();
    let actual = simd_json::from_slice::<Point>(&mut input);
    assert!(actual.is_err(), "extra struct field accepted: {actual:?}");
}

#[test]
fn object_keys_round_trip() {
    for key in [
        "quote\"key",
        "back\\slash",
        "line\nbreak",
        "tab\tkey",
        "nul\0key",
        "héllo😀",
    ] {
        let src = serde_json::json!({key: 1});
        let text = src.to_string();
        let owned = simd_json::to_owned_value(&mut text.clone().into_bytes()).unwrap();
        let mut input = text.clone().into_bytes();
        let borrowed = simd_json::to_borrowed_value(&mut input).unwrap();
        let mut input2 = text.into_bytes();
        let tape = simd_json::to_tape(&mut input2).unwrap();
        for encoded in [
            owned.encode(),
            owned.encode_pp(),
            borrowed.encode(),
            borrowed.encode_pp(),
            tape.as_value().encode(),
            tape.as_value().encode_pp(),
        ] {
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&encoded).ok(),
                Some(src.clone()),
                "key {key:?} encoded {encoded:?}"
            );
        }
    }
}

#[cfg(feature = "wide")]
#[test]
fn wide_tape_accessors() {
    let n = u64::MAX as u128 + 1;
    let mut input = n.to_string().into_bytes();
    let tape = simd_json::to_tape(&mut input).unwrap();
    assert_eq!(tape.as_value().as_u128(), Some(n));
    assert_eq!(tape.as_value().as_i128(), Some(n as i128));
}

#[cfg(feature = "wide")]
#[test]
fn wide_integer_as_float() {
    let mut input = b"18446744073709551616".to_vec();
    assert_eq!(
        simd_json::from_slice::<f64>(&mut input).unwrap(),
        18446744073709551616.0
    );
}

#[test]
fn serde_reference_control() {
    assert!(serde_json::from_str::<(u64, u64)>("[1,2,3]").is_err());
    assert!(serde_json::from_str::<((u64, u64), u64)>("[[1,2,3],4]").is_err());
}

#[test]
fn reject_large_integer_suffix() {
    for text in [
        "1234567890123456789x",
        "-123456789012345678x",
        "18446744073709551615x",
        "[1234567890123456789x]",
    ] {
        assert!(serde_json::from_str::<serde_json::Value>(text).is_err());
        let actual = simd_json::to_owned_value(&mut text.as_bytes().to_vec());
        assert!(
            actual.is_err(),
            "invalid number {text} accepted as {actual:?}"
        );
    }
}

#[test]
fn reject_invalid_surrogate_pair() {
    for text in [r#""\ud800\ue000""#, r#""\udbff\uffff""#] {
        assert!(serde_json::from_str::<String>(text).is_err());
        let actual = simd_json::from_slice::<String>(&mut text.as_bytes().to_vec());
        assert!(
            actual.is_err(),
            "invalid surrogate pair {text} accepted as {actual:?}"
        );
    }
}

#[test]
fn large_exponent_does_not_wrap() {
    for (text, expected) in [
        ("1e4294967296", None),
        ("1e-4294967296", Some(0.0)),
        ("1e4294967297", None),
        ("1e-4294967297", Some(0.0)),
    ] {
        let actual = simd_json::from_slice::<f64>(&mut text.as_bytes().to_vec());
        assert_eq!(actual.ok(), expected, "exponent wrapped for {text}");
    }
}
