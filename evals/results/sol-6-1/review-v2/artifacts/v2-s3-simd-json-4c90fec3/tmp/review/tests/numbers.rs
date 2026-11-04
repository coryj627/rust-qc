#[test]
fn long_numbers_require_number_terminator() {
    let mut mismatches = 0;
    for input in [
        "1234567890123456789x",
        "12345678901234567890.0x",
        "[1234567890123456789x]",
        "-123456789012345678x",
    ] {
        let expected = serde_json::from_str::<serde_json::Value>(input);
        let actual = simd_json::to_owned_value(&mut input.as_bytes().to_vec());
        println!("input={input}, expected={expected:?}, actual={actual:?}");
        if actual.is_ok() {
            mismatches += 1;
        }
    }
    assert_eq!(mismatches, 0);
}

#[test]
fn exponent_must_not_wrap() {
    let mut mismatches = 0;
    for input in [
        "1e4294967296",
        "1e-4294967296",
        "1e4294967297",
        "1e-4294967297",
        "-1.5e4294967296",
    ] {
        let expected = input.parse::<f64>().unwrap();
        let actual = simd_json::from_slice::<f64>(&mut input.as_bytes().to_vec());
        println!("input={input}, expected={expected:?}, actual={actual:?}");
        if expected.is_finite() {
            if !actual.is_ok_and(|v| v.to_bits() == expected.to_bits()) {
                mismatches += 1;
            }
        } else if actual.is_ok() {
            mismatches += 1;
        }
    }
    assert_eq!(mismatches, 0);
}

#[test]
fn ordinary_floats_match_std_parse() {
    let mut seed = 2134_u64;
    for i in 0..50_000 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let n = f64::from_bits(seed);
        if !n.is_finite() {
            continue;
        }
        let input = format!("{n:e}");
        let actual = simd_json::from_slice::<f64>(&mut input.as_bytes().to_vec()).unwrap();
        assert_eq!(actual.to_bits(), n.to_bits(), "{input} iteration {i}");
    }
}
