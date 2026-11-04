use simd_json::prelude::*;
use std::collections::BTreeMap;

#[test]
fn strings_across_simd_boundaries_and_reused_buffers() {
    let mut buffers = simd_json::Buffers::new(0);
    for len in [
        0, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129, 1024, 2,
    ] {
        for ending in ["plain", "quote\"slash\\", "héllo😀\n\t\u{0}"] {
            let expected = format!("{}{ending}", "a".repeat(len));
            let json = serde_json::to_vec(&expected).unwrap();
            let mut bytes = json.clone();
            let actual = simd_json::to_owned_value_with_buffers(&mut bytes, &mut buffers).unwrap();
            assert_eq!(actual.as_str(), Some(expected.as_str()));
            let mut bytes = json.clone();
            let tape = simd_json::to_tape_with_buffers(&mut bytes, &mut buffers).unwrap();
            assert_eq!(tape.as_value().as_str(), Some(expected.as_str()));
            let mut bytes = json;
            let actual: String =
                simd_json::serde::from_slice_with_buffers(&mut bytes, &mut buffers).unwrap();
            assert_eq!(actual, expected);
        }
    }
    assert!(simd_json::to_tape_with_buffers(&mut b"[1,".to_vec(), &mut buffers).is_err());
    assert_eq!(
        simd_json::to_owned_value_with_buffers(&mut b"[2]".to_vec(), &mut buffers)
            .unwrap()
            .get_idx(0)
            .unwrap()
            .as_u64(),
        Some(2)
    );
}

#[test]
fn lazy_mutation_preserves_existing_values() {
    let mut input = br#"{"name":"caf\u00e9","list":[1,{"item":2}]}"#.to_vec();
    let tape = simd_json::to_tape(&mut input).unwrap();
    let mut lazy = simd_json::value::lazy::Value::from_tape(tape.as_value());
    assert!(lazy.is_tape());
    assert_eq!(
        lazy.get("list")
            .unwrap()
            .get_idx(1)
            .unwrap()
            .get("item")
            .unwrap()
            .as_u64(),
        Some(2)
    );
    lazy.insert("extra", 3).unwrap();
    assert!(lazy.is_value());
    assert_eq!(lazy.get("name").unwrap().as_str(), Some("café"));
    assert_eq!(lazy.get("extra").unwrap().as_u64(), Some(3));
}

#[cfg(feature = "dedup")]
#[test]
fn duplicate_keys_keep_last_for_small_and_large_maps() {
    for count in [1, 32, 33, 100] {
        let fields = (0..count)
            .map(|n| format!("\"key{n}\":{n}"))
            .collect::<Vec<_>>()
            .join(",");
        let text = format!("{{{fields},\"key0\":999}}");
        let mut bytes = text.clone().into_bytes();
        let owned = simd_json::to_owned_value(&mut bytes).unwrap();
        assert_eq!(owned.as_object().unwrap().len(), count);
        assert_eq!(owned["key0"].as_u64(), Some(999));
        let mut bytes = text.into_bytes();
        let borrowed = simd_json::to_borrowed_value(&mut bytes).unwrap();
        assert_eq!(borrowed.as_object().unwrap().len(), count);
        assert_eq!(borrowed["key0"].as_u64(), Some(999));
    }
}

#[cfg(feature = "known")]
#[test]
fn known_keys_work_across_map_storage_modes() {
    let key = simd_json::KnownKey::from("key0");
    for count in [1, 32, 33, 100] {
        let expected = (0..count)
            .map(|n| (format!("key{n}"), n))
            .collect::<BTreeMap<_, _>>();
        let mut value = simd_json::serde::to_borrowed_value(&expected).unwrap();
        assert_eq!(key.lookup(&value).unwrap().as_u64(), Some(0));
        *key.lookup_mut(&mut value).unwrap() = 999.into();
        assert_eq!(key.lookup(&value).unwrap().as_u64(), Some(999));
    }
}

#[test]
fn unicode_pair_boundaries() {
    for (text, expected) in [
        (r#""\ud800\udc00""#, "\u{10000}"),
        (r#""\udbff\udfff""#, "\u{10ffff}"),
        (r#""\u0000""#, "\0"),
        (r#""\uffff""#, "\u{ffff}"),
    ] {
        let actual = simd_json::from_slice::<String>(&mut text.as_bytes().to_vec()).unwrap();
        assert_eq!(actual, expected);
    }
}
