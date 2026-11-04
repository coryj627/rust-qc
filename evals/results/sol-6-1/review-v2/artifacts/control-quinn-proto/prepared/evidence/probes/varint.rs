use super::*;

#[test]
fn negative_control_false_1_u128_conversion_checks_before_narrowing() {
    let max = (1u128 << 62) - 1;
    assert_eq!(VarInt::try_from(max).unwrap().into_inner(), max as u64);
    for value in [1u128 << 62, u64::MAX as u128, 1u128 << 64, (1u128 << 64) + 7, u128::MAX] {
        assert_eq!(VarInt::try_from(value), Err(VarIntBoundsExceeded));
    }
    assert_eq!(VarInt::try_from(7u128).unwrap().into_inner(), 7);
    println!("false-1: 2^62-1 accepted; 2^62, u64::MAX, 2^64, 2^64+7 and u128::MAX rejected; 7 retained");
}
