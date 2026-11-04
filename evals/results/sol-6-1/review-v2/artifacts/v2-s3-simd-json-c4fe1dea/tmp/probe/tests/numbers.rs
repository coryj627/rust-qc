use simd_json::prelude::*;

#[test]
fn float_value_is_independent_of_byte_offset() {
    let numbers = [
        "123456789012345678901.25",
        "1.2345678901234567890123456789",
        "0.000000000000000000000000123456789",
        "123456789012345678901234567890e-10",
        "0.123456789012345678901234567890",
    ];
    let mut failures = Vec::new();
    for number in numbers {
        let expected: f64 = number.parse().unwrap();
        for prefix in 0..140 {
            let mut input = format!("{}{}", " ".repeat(prefix), number).into_bytes();
            let actual = simd_json::from_slice::<f64>(&mut input).unwrap();
            if actual.to_bits() != expected.to_bits() {
                failures.push((prefix, number, actual, expected));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "offset-dependent float parsing: {failures:?}"
    );
}

#[test]
fn floats_round_trip() {
    // Fixed values near common decimal precision boundaries, including subnormals.
    for f in [
        0.0_f64,
        -0.0,
        1.0,
        -1.0,
        1.1,
        1e-300,
        1e300,
        f64::MIN_POSITIVE,
        f64::MAX,
        f64::from_bits(1),
        9007199254740992.0,
    ] {
        let text = simd_json::to_string(&f).unwrap();
        let actual = simd_json::from_slice::<f64>(&mut text.clone().into_bytes()).unwrap();
        assert_eq!(actual.to_bits(), f.to_bits(), "failed roundtrip {text}");
        let value = simd_json::OwnedValue::from(f);
        let encoded = value.encode();
        assert_eq!(encoded.parse::<f64>().unwrap().to_bits(), f.to_bits());
    }
}
