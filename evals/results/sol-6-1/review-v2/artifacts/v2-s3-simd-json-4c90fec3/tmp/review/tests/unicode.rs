#[test]
fn unicode_surrogate_pairs_require_low_surrogate() {
    let mut mismatches = 0;
    for input in [
        r#""\ud800""#,
        r#""\ud800x""#,
        r#""\ud800\ue000""#,
        r#""\ud800\uffff""#,
        r#""\udbff\ue000""#,
    ] {
        let expected = serde_json::from_str::<String>(input);
        let actual = simd_json::from_slice::<String>(&mut input.as_bytes().to_vec());
        println!("input={input}, expected={expected:?}, actual={actual:?}");
        if actual.is_ok() {
            mismatches += 1;
        }
    }
    assert_eq!(mismatches, 0);
}

#[test]
fn unicode_strings_at_simd_boundaries() {
    for n in 0..140 {
        for suffix in [
            "é",
            "😀",
            "\\n",
            "\\u0000",
            "\\ud800\\udc00",
            "\\udbff\\udfff",
        ] {
            let input = format!("\"{}{suffix}end\"", "x".repeat(n));
            let expected = serde_json::from_str::<String>(&input).unwrap();
            let actual = simd_json::from_slice::<String>(&mut input.as_bytes().to_vec()).unwrap();
            assert_eq!(actual, expected, "{input}");
        }
    }
}
