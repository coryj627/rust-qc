fn serializable<T:serde::Serialize + ?Sized>(_: &T) {}
fn main() {
    let v=heapless::Vec::<u8,4>::new();
    serializable(&v);
    serializable(v.as_view());
}
