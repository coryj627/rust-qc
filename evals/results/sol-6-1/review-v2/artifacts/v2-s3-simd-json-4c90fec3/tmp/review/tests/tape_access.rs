use simd_json::prelude::*;

#[test]
fn named_array_accessor_preserves_all_elements() {
    let mut mismatches = 0;
    for input in [
        r#"{"a":[]}"#,
        r#"{"a":[1]}"#,
        r#"{"a":[1,2]}"#,
        r#"{"a":[[1,2]]}"#,
    ] {
        let mut bytes = input.as_bytes().to_vec();
        let tape = simd_json::to_tape(&mut bytes).unwrap();
        let root = tape.as_value();
        let expected = root
            .get("a")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.encode())
            .collect::<Vec<_>>();
        let actual = std::panic::catch_unwind(|| {
            root.get_array("a")
                .unwrap()
                .iter()
                .map(|v| v.encode())
                .collect::<Vec<_>>()
        });
        println!("input={input}, expected={expected:?}, actual={actual:?}");
        if !actual.is_ok_and(|actual| actual == expected) {
            mismatches += 1;
        }
    }
    assert_eq!(mismatches, 0);
}

#[test]
fn named_object_accessor_preserves_all_entries() {
    let mut input = br#"{"o":{"a":1,"b":2}}"#.to_vec();
    let tape = simd_json::to_tape(&mut input).unwrap();
    let root = tape.as_value();
    let expected = root
        .get("o")
        .unwrap()
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k, v.encode()))
        .collect::<Vec<_>>();
    let actual = root
        .get_object("o")
        .unwrap()
        .iter()
        .map(|(k, v)| (k, v.encode()))
        .collect::<Vec<_>>();
    println!("expected={expected:?}, actual={actual:?}");
    assert_eq!(actual, expected);
}

#[cfg(feature = "int128")]
#[test]
fn tape_preserves_128_bit_scalar_access() {
    let n = (u128::from(u64::MAX) + 1).to_string();
    let mut bytes = n.as_bytes().to_vec();
    let tape = simd_json::to_tape(&mut bytes).unwrap();
    let root = tape.as_value();
    println!("type={:?}, u128={:?}", root.value_type(), root.as_u128());
    assert_eq!(root.as_u128(), Some(u128::from(u64::MAX) + 1));
}

#[test]
fn tape_casts_integer_to_float() {
    let mut input = b"42".to_vec();
    let tape = simd_json::to_tape(&mut input).unwrap();
    assert_eq!(tape.as_value().cast_f64(), Some(42.0));
}
