fn assert_serialize<T: serde::Serialize + ?Sized>(_: &T) {}

fn main() {
    let v = heapless::Vec::<u8, 4>::from_slice(b"abc").unwrap();
    assert_serialize(&v);
    assert_serialize(v.as_view());
}
