#[test]
fn serialize_vec_view() {
    fn serializable<T: serde::Serialize + ?Sized>(_: &T) {}
    let vec = heapless::Vec::<u8, 4>::from_slice(b"abc").unwrap();
    serializable(&vec);
    serializable(vec.as_view());
}
