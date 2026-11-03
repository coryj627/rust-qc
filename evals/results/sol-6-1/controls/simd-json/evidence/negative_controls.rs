use simd_json::{prelude::*, BorrowedValue, Buffers};

#[test]
fn decoy_1_trailing_root_is_rejected() {
    let roots = ["true", "false", "null", "\"text\"", "17", "-17", "1.25", "[]", "{}"];
    let tails = [" false", " 23", " {}", " []", " \"extra\""];
    for root in roots {
        let valid = format!("{root} \r\n\t");
        assert!(simd_json::to_owned_value(&mut valid.into_bytes()).is_ok());
        for tail in tails {
            let input = format!("{root}{tail}");
            let result = simd_json::to_owned_value(&mut input.clone().into_bytes());
            assert!(result.is_err(), "accepted {input:?}");
        }
    }
    println!("decoy-1: rejected all 45 two-root documents; accepted all 9 whitespace-only suffixes");
}

#[derive(serde::Deserialize, Debug, PartialEq)]
struct Keep {
    keep: u64,
}

#[test]
fn decoy_2_ignored_nested_field_is_drained() {
    let unknowns = [
        "[]", "{}", "[1,2,3]", "{\"a\":1,\"b\":2}",
        "[{\"a\":[1,{\"b\":[null,false,\"x\"]}]},[],{\"c\":3}]",
        "{\"outer\":[{\"inner\":[]},[{},[true]]],\"tail\":\"done\"}",
    ];
    for unknown in unknowns {
        let doc = format!("{{\"ignored\":{unknown},\"keep\":42}}");
        let result: Keep = simd_json::serde::from_slice(&mut doc.into_bytes()).unwrap();
        assert_eq!(result, Keep { keep: 42 });
    }
    println!("decoy-2: all 6 ignored containers, including nested arrays/objects, consumed fully; following keep=42");
}

#[test]
fn false_1_small_integer_ranges_are_checked() {
    for (text, expected) in [("0", 0u8), ("255", 255u8)] {
        let actual: u8 = simd_json::serde::from_slice(&mut text.as_bytes().to_vec()).unwrap();
        assert_eq!(actual, expected);
    }
    for text in ["256", "511", "-1", "18446744073709551615"] {
        assert!(simd_json::serde::from_slice::<u8>(&mut text.as_bytes().to_vec()).is_err(), "u8 accepted {text}");
    }
    for (text, expected) in [("-128", -128i8), ("127", 127i8)] {
        let actual: i8 = simd_json::serde::from_slice(&mut text.as_bytes().to_vec()).unwrap();
        assert_eq!(actual, expected);
    }
    for text in ["-129", "128", "255", "9223372036854775807"] {
        assert!(simd_json::serde::from_slice::<i8>(&mut text.as_bytes().to_vec()).is_err(), "i8 accepted {text}");
    }
    println!("false-1: u8/i8 accepted all 4 boundary values and rejected all 8 out-of-range integers");
}

#[test]
fn false_2_dom_object_keys_are_escaped() {
    let keys = ["quote\"key", "back\\slash", "line\nfeed", "nul\0key", "tab\tkey", "é🔑"];
    for key in keys {
        let mut original = serde_json::Map::new();
        original.insert(key.into(), serde_json::json!(17));
        let expected = serde_json::Value::Object(original);
        let doc = serde_json::to_vec(&expected).unwrap();
        let owned = simd_json::to_owned_value(&mut doc.clone()).unwrap();
        let mut data = doc;
        let borrowed = simd_json::to_borrowed_value(&mut data).unwrap();
        for encoded in [owned.encode(), owned.encode_pp(), borrowed.encode(), borrowed.encode_pp()] {
            let roundtrip: serde_json::Value = serde_json::from_str(&encoded).unwrap();
            assert_eq!(roundtrip, expected, "key={key:?}, output={encoded:?}");
        }
    }
    println!("false-2: compact/pretty owned/borrowed DOM output round-tripped all 6 special keys (24 encodings)");
}

#[test]
fn false_3_reused_buffers_reset_structural_state() {
    let mut buffers = Buffers::new(0);
    let mut cases: Vec<String> = vec![
        format!("[{}]", (0..100).map(|n| format!("{{\"n\":{n},\"s\":\"x\\ny\"}}")).collect::<Vec<_>>().join(",")),
        "{}".into(), "[]".into(), "0".into(), "\"short\"".into(),
        "[1,".into(), "true".into(), "\"unterminated".into(), "false".into(),
        " ".into(), "{\"final\":42}".into(),
    ];
    let count = cases.len();
    for _ in 0..3 {
        for text in &mut cases {
            let expected = serde_json::from_str::<serde_json::Value>(text);
            let actual = simd_json::to_owned_value_with_buffers(&mut text.as_bytes().to_vec(), &mut buffers);
            match expected {
                Ok(value) => {
                    let actual = actual.unwrap();
                    let result: serde_json::Value = actual.try_into().unwrap();
                    assert_eq!(result, value, "buffer reuse changed {text:?}");
                }
                Err(_) => assert!(actual.is_err(), "accepted invalid {text:?}"),
            }
        }
    }
    println!("false-3: {} sequential parses on one Buffers matched fresh reference results, including recovery from 3 error cases per cycle", count * 3);
}

#[test]
fn false_4_raw_control_bytes_in_strings_are_rejected() {
    let positions = [0usize, 1, 15, 16, 31, 32, 62, 63, 64, 65, 127, 128];
    for pad in positions {
        for control in 0u8..=31 {
            let mut raw = vec![b'"'];
            raw.extend(std::iter::repeat_n(b'a', pad));
            raw.push(control);
            raw.push(b'"');
            assert!(simd_json::to_owned_value(&mut raw).is_err(), "accepted control={control}, pad={pad}");
            let escaped = format!("\"{}\\u{control:04x}\"", "a".repeat(pad));
            let value = simd_json::to_owned_value(&mut escaped.into_bytes()).unwrap();
            let text = value.as_str().unwrap();
            assert_eq!(text.as_bytes()[pad], control);
        }
    }
    println!("false-4: rejected all 384 raw-control strings across SIMD boundaries; accepted all 384 escaped equivalents");
}

fn assert_owned_tree(value: &BorrowedValue<'_>) -> usize {
    match value {
        BorrowedValue::String(s) => {
            assert!(matches!(s, simd_json::cow::Cow::Owned(_)), "string still borrowed");
            1
        }
        BorrowedValue::Array(a) => a.iter().map(assert_owned_tree).sum(),
        BorrowedValue::Object(o) => o.iter().map(|(key, value)| {
            assert!(matches!(key, simd_json::cow::Cow::Owned(_)), "key still borrowed");
            1 + assert_owned_tree(value)
        }).sum(),
        BorrowedValue::Static(_) => 0,
    }
}

#[test]
fn false_5_into_static_owns_nested_strings_and_keys() {
    let mut data = br#"{"outer":[{"inner":"borrowed","escaped\nkey":"line\nvalue"},"leaf"],"last":"tail"}"#.to_vec();
    let borrowed = simd_json::to_borrowed_value(&mut data).unwrap();
    assert!(matches!(borrowed["last"], BorrowedValue::String(simd_json::cow::Cow::Borrowed(_))));
    let static_value: BorrowedValue<'static> = borrowed.into_static();
    let checked = assert_owned_tree(&static_value);
    assert_eq!(checked, 8);
    // All leaves and keys were proven owned before the original input is touched.
    data.fill(b'x');
    drop(data);
    let expected = serde_json::json!({"outer":[{"inner":"borrowed","escaped\nkey":"line\nvalue"},"leaf"],"last":"tail"});
    let roundtrip: serde_json::Value = serde_json::from_str(&static_value.encode()).unwrap();
    assert_eq!(roundtrip, expected);
    println!("false-5: into_static changed all 8 nested strings/keys to owned Cows; value stayed intact after input overwrite/drop");
}
