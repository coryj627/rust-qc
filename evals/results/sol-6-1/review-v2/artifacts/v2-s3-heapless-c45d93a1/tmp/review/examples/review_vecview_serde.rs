fn require_serialize<T: serde::Serialize + ?Sized>(_: &T) {}
fn main() {
    let v = heapless::Vec::<u8, 4>::from_slice(b"ab").unwrap();
    require_serialize(&v);
    require_serialize(v.as_view());
}
