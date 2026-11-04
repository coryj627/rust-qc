use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Serialize, serde::Deserialize, PartialEq)]
enum Message {
    Pair(u8, u8),
    Empty(),
    Object { x: u8 },
    EmptyObject {},
}

#[test]
fn pretty_tuple_variants_roundtrip() {
    let mut mismatches = 0;
    for value in [
        Message::Pair(1, 2),
        Message::Empty(),
        Message::EmptyObject {},
    ] {
        let json = simd_json::to_string_pretty(&value).unwrap();
        let actual = serde_json::from_str::<Message>(&json);
        println!("value={value:?}, json={json:?}, actual={actual:?}");
        if !actual.is_ok_and(|actual| actual == value) {
            mismatches += 1;
        }
    }
    assert_eq!(mismatches, 0);
}

#[derive(Serialize)]
struct Flat {
    #[serde(flatten)]
    extra: BTreeMap<String, u8>,
}

#[test]
fn pretty_empty_flattened_map_roundtrip() {
    let value = Flat {
        extra: BTreeMap::new(),
    };
    let json = simd_json::to_string_pretty(&value).unwrap();
    println!("json={json:?}");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&json).unwrap(),
        serde_json::json!({})
    );
}

#[test]
fn tuple_variant_does_not_change_parent_indentation() {
    let value = [Message::Pair(1, 2), Message::Object { x: 3 }];
    let json = simd_json::to_string_pretty(&value).unwrap();
    println!("json={json:?}");
    assert_eq!(serde_json::from_str::<Vec<Message>>(&json).unwrap(), value);
}
