use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[test]
fn integer_map_keys_survive_value_conversion() {
    let expected = BTreeMap::from([(42_u32, "answer".to_string())]);
    let owned = simd_json::serde::to_owned_value(&expected).unwrap();
    let by_ref: BTreeMap<u32, String> = simd_json::serde::from_refowned_value(&owned).unwrap();
    assert_eq!(by_ref, expected);
    let actual = simd_json::serde::from_owned_value::<BTreeMap<u32, String>>(owned);
    assert_eq!(actual.unwrap(), expected);
}

#[test]
fn integer_map_keys_survive_borrowed_value_conversion() {
    let expected = BTreeMap::from([(42_u32, "answer".to_string())]);
    let borrowed = simd_json::serde::to_borrowed_value(&expected).unwrap();
    let by_ref: BTreeMap<u32, String> =
        simd_json::serde::from_refborrowed_value(&borrowed).unwrap();
    assert_eq!(by_ref, expected);
    let actual = simd_json::serde::from_borrowed_value::<BTreeMap<u32, String>>(borrowed);
    assert_eq!(actual.unwrap(), expected);
}

#[test]
fn pretty_tuple_variant_round_trip() {
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    enum Event {
        Point(u32, u32),
    }
    let expected = Event::Point(1, 2);
    let compact = simd_json::to_string(&expected).unwrap();
    assert_eq!(serde_json::from_str::<Event>(&compact).unwrap(), expected);
    let pretty = simd_json::to_string_pretty(&expected).unwrap();
    let actual = serde_json::from_str::<Event>(&pretty);
    assert_eq!(
        actual.unwrap_or_else(|e| panic!("invalid output {pretty:?}: {e}")),
        expected
    );
}

#[test]
fn pretty_empty_flattened_map_is_valid() {
    #[derive(Serialize)]
    struct Attributes {
        #[serde(flatten)]
        fields: BTreeMap<String, String>,
    }
    let expected = Attributes {
        fields: BTreeMap::new(),
    };
    let pretty = simd_json::to_string_pretty(&expected).unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&pretty).ok(),
        Some(serde_json::json!({})),
        "invalid output {pretty:?}"
    );
}

struct EmptyFiltered;
impl Serialize for EmptyFiltered {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq([1, 2, 3].into_iter().filter(|n| *n > 10))
    }
}

#[test]
fn unknown_length_empty_sequence_is_valid() {
    for text in [
        simd_json::to_string(&EmptyFiltered).unwrap(),
        simd_json::to_string_pretty(&EmptyFiltered).unwrap(),
    ] {
        assert_eq!(
            serde_json::from_str::<Vec<u32>>(&text).ok(),
            Some(vec![]),
            "invalid output {text:?}"
        );
    }
}

#[test]
fn standard_enum_forms_round_trip() {
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    enum Event {
        Unit,
        New(u64),
        Tuple(u64, String),
        Struct { x: u64 },
    }
    for expected in [
        Event::Unit,
        Event::New(12),
        Event::Tuple(34, "hello".into()),
        Event::Struct { x: 56 },
    ] {
        let mut text = simd_json::to_vec(&expected).unwrap();
        assert_eq!(simd_json::from_slice::<Event>(&mut text).unwrap(), expected);
    }
}
