use super::*;

#[test]
fn negative_control_false_5_zero_idle_timeout_preserves_other_peers_limit() {
    let zero = Some(VarInt::from_u32(0));
    let finite = Some(VarInt::from_u32(5000));
    let expected = Some(Duration::from_millis(5000));
    assert_eq!(negotiate_max_idle_timeout(zero, finite), expected);
    assert_eq!(negotiate_max_idle_timeout(finite, zero), expected);
    assert_eq!(negotiate_max_idle_timeout(None, finite), expected);
    assert_eq!(negotiate_max_idle_timeout(finite, None), expected);
    assert_eq!(negotiate_max_idle_timeout(zero, zero), None);
    assert_eq!(negotiate_max_idle_timeout(zero, None), None);
    assert_eq!(negotiate_max_idle_timeout(Some(VarInt::from_u32(9000)), finite), expected);
    println!("false-5: zero/5000ms and None/5000ms negotiate 5000ms in either order; both disabled negotiate None; two finite limits use minimum");
}
