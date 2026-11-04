use simd_json::prelude::*;

#[test]
fn buffers_reuse_across_sizes_and_errors() {
    let mut buffers = simd_json::Buffers::new(0);
    for input in [
        "null".to_string(),
        format!("[{}]", vec!["123"; 4096].join(",")),
        "{broken".to_string(),
        "[1,{},[],\"é😀\\n\"]".to_string(),
        serde_json::to_string(&"aé😀\n".repeat(32_768)).unwrap(),
        "{}".to_string(),
    ] {
        let expected = serde_json::from_str::<serde_json::Value>(&input);
        let actual =
            simd_json::to_owned_value_with_buffers(&mut input.as_bytes().to_vec(), &mut buffers);
        match (expected, actual) {
            (Ok(expected), Ok(actual)) => assert_eq!(
                serde_json::from_str::<serde_json::Value>(&actual.encode()).unwrap(),
                expected
            ),
            (Err(_), Err(_)) => (),
            (expected, actual) => panic!("expected={expected:?} actual={actual:?}"),
        }
    }
}

#[test]
fn scalar_integer_boundaries_and_float_boundaries() {
    for input in [
        "0",
        "-0",
        "18446744073709551615",
        "-9223372036854775808",
        "2.2250738585072014e-308",
        "1.7976931348623157e308",
        "5e-324",
        "0.0e999",
        "-0.0e999",
        "1e-999",
        "-1e-999",
        "1e309",
    ] {
        let expected = input.parse::<f64>().unwrap();
        let actual = simd_json::from_slice::<f64>(&mut input.as_bytes().to_vec());
        if expected.is_finite() {
            assert_eq!(actual.unwrap(), expected, "{input}");
        } else {
            assert!(actual.is_err(), "{input}");
        }
    }
}

#[test]
fn dom_tape_and_serde_roundtrip_nested_object() {
    let mut object = serde_json::Map::new();
    for n in 0..100 {
        object.insert(
            format!("key{n}é\n"),
            serde_json::json!([n, null, true, {"nested": [1,2,3]}]),
        );
    }
    let expected = serde_json::Value::Object(object);
    let input = serde_json::to_vec(&expected).unwrap();
    let owned = simd_json::to_owned_value(&mut input.clone()).unwrap();
    let mut borrowed_input = input.clone();
    let borrowed = simd_json::to_borrowed_value(&mut borrowed_input).unwrap();
    let mut tape_input = input.clone();
    let tape = simd_json::to_tape(&mut tape_input).unwrap();
    for encoded in [
        owned.encode(),
        owned.encode_pp(),
        borrowed.encode(),
        borrowed.encode_pp(),
        tape.as_value().encode(),
        tape.as_value().encode_pp(),
    ] {
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&encoded).unwrap(),
            expected
        );
    }
    assert_eq!(
        simd_json::from_slice::<serde_json::Value>(&mut input.clone()).unwrap(),
        expected
    );
}

#[cfg(feature = "dedup")]
#[test]
fn duplicate_keys_last_value_wins_for_all_map_sizes() {
    for n in [0, 1, 31, 32, 33, 99] {
        let mut fields = (0..n)
            .map(|i| format!("\"key{i}\":{i}"))
            .collect::<Vec<_>>();
        fields.extend(["\"duplicate\":1".into(), "\"duplicate\":2".into()]);
        let input = format!("{{{}}}", fields.join(","));
        let actual = simd_json::to_owned_value(&mut input.as_bytes().to_vec()).unwrap();
        assert_eq!(actual.get_u64("duplicate"), Some(2));
        assert_eq!(actual.as_object().unwrap().len(), n + 1);
    }
}
