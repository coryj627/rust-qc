use heapless::{Vec, VecView};
use serde::Serialize;
fn accepts_serializable<T: Serialize + ?Sized>(_: &T) {}
fn main() {
    let v = Vec::<u8, 4>::from_array([1, 2]);
    accepts_serializable(&v);
    let view: &VecView<u8> = v.as_view();
    accepts_serializable(view);
}
