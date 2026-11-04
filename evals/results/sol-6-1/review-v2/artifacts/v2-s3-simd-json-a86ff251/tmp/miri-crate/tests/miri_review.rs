#[test]
fn valid_scalar() {
    let mut input = b"1".to_vec();
    assert!(simd_json::to_tape(&mut input).is_ok());
}
