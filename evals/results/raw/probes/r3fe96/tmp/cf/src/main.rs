use heapless::{HistoryBuf, Vec, VecView, deque::{Deque, DequeView}};
fn ser<T: serde::Serialize + ?Sized>(_: &T) {}
struct NotCopy;
fn main() {
    let d: Deque<u8, 2> = Deque::new();
    let dv: &DequeView<u8> = &d;
    ser(dv);
    let v: Vec<u8, 2> = Vec::new();
    let vv: &VecView<u8> = &v;
    ser(vv);
    let h: HistoryBuf<NotCopy, 2> = HistoryBuf::new();
    let _ = h.as_view();
}
