use serde::Deserialize;
use simd_json::prelude::*;
use std::collections::BTreeMap;

#[test]
fn decoy_1_empty_known_length_sequence() {
    let output = simd_json::to_string(&Vec::<u8>::new()).unwrap();
    assert_eq!(output, "[]");
    let array_output = simd_json::to_string(&[0u8; 0]).unwrap();
    assert_eq!(array_output, "[]");
    let nested_output = simd_json::to_string(&vec![Vec::<u8>::new(), vec![1]]).unwrap();
    assert_eq!(nested_output, "[[],[1]]");
    println!("decoy-1: empty Vec={output}; empty array={array_output}; nested={nested_output}");
}

#[test]
fn decoy_2_ignored_nested_fields_are_consumed() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Keep { kept: u64 }
    let cases = [
        r#"{"unused":{"items":[1,{"deep":[true,null,"x"]}]},"kept":9}"#,
        r#"{"unused":[1,{"deep":[true,null,"x"]}],"kept":9}"#,
        r#"{"kept":9,"unused":{"items":[1,{"deep":[true,null,"x"]}]}}"#,
    ];
    for input in cases {
        let mut bytes = input.as_bytes().to_vec();
        let result: Keep = simd_json::from_slice(&mut bytes).unwrap();
        assert_eq!(result, Keep { kept: 9 });
        println!("decoy-2: {input} -> {result:?}");
    }
}

#[test]
fn false_1_u8_range_is_checked() {
    for (input, expected) in [("0", 0u8), ("255", 255u8)] {
        let result: u8 = simd_json::from_slice(&mut input.as_bytes().to_vec()).unwrap();
        assert_eq!(result, expected);
        println!("false-1: u8({input}) -> {result}");
    }
    for input in ["-1", "256"] {
        let result = simd_json::from_slice::<u8>(&mut input.as_bytes().to_vec());
        assert!(result.is_err(), "unexpected success for {input}: {result:?}");
        println!("false-1: u8({input}) -> {}", result.unwrap_err());
    }
}

#[test]
fn false_2_second_root_is_rejected() {
    for input in ["true false", "{} {}", "[1] [2]", "42 43", r#""a" "b""#] {
        let result = simd_json::from_slice::<serde_json::Value>(&mut input.as_bytes().to_vec());
        assert!(result.is_err(), "unexpected success for {input}: {result:?}");
        println!("false-2: {input} -> {}", result.unwrap_err());
    }
    let accepted: serde_json::Value = simd_json::from_slice(&mut b"true \n\t".to_vec()).unwrap();
    assert_eq!(accepted, serde_json::Value::Bool(true));
    println!("false-2: true plus trailing JSON whitespace -> {accepted}");
}

#[test]
fn false_3_buffers_do_not_keep_old_structural_indexes() {
    let mut buffers = simd_json::Buffers::new(1);
    let first = format!(r#"{{"old":[1,2,{{"nested":[3,4,5]}}],"padding":"{}"}}"#, "x".repeat(140));
    let cases = [first.as_str(), "[]", "null", "7", r#"{"ok":true}"#, "[9]"];
    for input in cases {
        let buffered: serde_json::Value = simd_json::serde::from_slice_with_buffers(
            &mut input.as_bytes().to_vec(), &mut buffers,
        ).unwrap();
        let fresh: serde_json::Value = simd_json::from_slice(&mut input.as_bytes().to_vec()).unwrap();
        assert_eq!(buffered, fresh);
        println!("false-3: reused Buffers, input bytes={}, result={buffered}", input.len());
    }
}

#[test]
fn false_4_signed_integer_map_keys_deserialize() {
    let mut input = br#"{"-12":7,"5":8}"#.to_vec();
    let map: BTreeMap<i32, u8> = simd_json::from_slice(&mut input).unwrap();
    assert_eq!(map, BTreeMap::from([(-12, 7), (5, 8)]));
    let output = simd_json::to_string(&map).unwrap();
    assert_eq!(output, r#"{"-12":7,"5":8}"#);
    let roundtrip: BTreeMap<i32, u8> = simd_json::from_slice(&mut output.as_bytes().to_vec()).unwrap();
    assert_eq!(roundtrip, map);
    println!("false-4: signed integer map={map:?}; serialized={output}; roundtrip equal");
}

#[test]
fn false_5_array_index_skips_whole_nested_values() {
    for input in ["[10,[20,30],40]", "[10,[],40]", r#"[10,{"x":[20,30]},40]"#] {
        let mut bytes = input.as_bytes().to_vec();
        let tape = simd_json::to_tape(&mut bytes).unwrap();
        let value = tape.as_value();
        let array = value.as_array().unwrap();
        assert_eq!(array.len(), 3);
        assert_eq!(array.get(0).and_then(|v| v.as_u64()), Some(10));
        assert_eq!(array.get(2).and_then(|v| v.as_u64()), Some(40));
        assert!(array.get(3).is_none());
        let leaves: Vec<_> = array.iter().map(|v| v.as_u64()).collect();
        assert_eq!(leaves, vec![Some(10), None, Some(40)]);
        if input == "[10,[20,30],40]" {
            let nested_value = array.get(1).unwrap();
            let nested = nested_value.as_array().unwrap();
            assert_eq!(nested.get(1).and_then(|v| v.as_u64()), Some(30));
        }
        println!("false-5: {input} -> len=3, get(2)=40, get(3)=None, scalar iteration={leaves:?}");
    }
}
