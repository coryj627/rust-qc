fn assert_eq_impl<T: Eq>() {}
#[test]
fn linear_map_eq_with_f32() {
    assert_eq_impl::<heapless::LinearMap<u8, f32, 4>>();
    let mut a: heapless::LinearMap<u8, f32, 4> = heapless::LinearMap::new();
    a.insert(1, f32::NAN).unwrap();
    println!("Eq type, a == a: {}", a == a);
    assert!(a == a);
}
