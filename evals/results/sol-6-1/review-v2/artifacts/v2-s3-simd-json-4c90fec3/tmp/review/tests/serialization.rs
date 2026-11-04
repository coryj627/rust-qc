use serde::Serialize;

#[derive(Debug, Serialize, serde::Deserialize, PartialEq)]
enum Renamed {
    #[serde(rename = "a\"b")]
    Unit,
    #[serde(rename = "x\ny")]
    Tuple(u8),
    #[serde(rename = "x\\y")]
    Object { a: u8 },
}

#[derive(Debug, Serialize, serde::Deserialize, PartialEq)]
struct EscapedField {
    #[serde(rename = "a\"b")]
    field: u8,
}

#[test]
fn serde_char_roundtrip() {
    for c in ['"', '\\', '\n', '\0', 'é', '😀'] {
        let expected = serde_json::to_string(&c).unwrap();
        let actual = simd_json::to_string(&c).unwrap();
        println!("c={c:?}, expected={expected:?}, actual={actual:?}");
        assert_eq!(actual, expected);
    }
}

#[test]
fn serde_renamed_fields_roundtrip() {
    let value = EscapedField { field: 1 };
    let expected = serde_json::to_string(&value).unwrap();
    let actual = simd_json::to_string(&value).unwrap();
    println!("expected={expected:?}, actual={actual:?}");
    assert_eq!(actual, expected);
}

#[test]
fn serde_renamed_variants_roundtrip() {
    for value in [Renamed::Unit, Renamed::Tuple(1), Renamed::Object { a: 1 }] {
        let expected = serde_json::to_string(&value).unwrap();
        let actual = simd_json::to_string(&value).unwrap();
        println!("expected={expected:?}, actual={actual:?}");
        assert_eq!(actual, expected);
    }
}

#[test]
fn serde_empty_unknown_length_sequence_roundtrip() {
    let value = std::iter::empty::<u8>();
    struct Sequence<I>(I);
    impl<I> Serialize for Sequence<I>
    where
        I: Clone + Iterator<Item = u8>,
    {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            use serde::ser::SerializeSeq;
            let mut seq = serializer.serialize_seq(None)?;
            for value in self.0.clone() {
                seq.serialize_element(&value)?;
            }
            seq.end()
        }
    }
    let expected = serde_json::to_string(&Sequence(value.clone())).unwrap();
    let actual = simd_json::to_string(&Sequence(value)).unwrap();
    println!("expected={expected:?}, actual={actual:?}");
    assert_eq!(actual, expected);
}
