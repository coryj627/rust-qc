use std::cell::Cell;
use std::panic::{catch_unwind, AssertUnwindSafe};

#[derive(Debug)]
struct CountDrop<'a> {
    id: usize,
    counts: &'a [Cell<usize>; 3],
    panic_once: &'a Cell<bool>,
}
impl Drop for CountDrop<'_> {
    fn drop(&mut self) {
        self.counts[self.id].set(self.counts[self.id].get() + 1);
        if self.id == 1 && self.panic_once.replace(false) {
            panic!("one destructor failure");
        }
    }
}
fn items<'a>(counts: &'a [Cell<usize>; 3], panic_once: &'a Cell<bool>) -> [CountDrop<'a>; 3] {
    [0, 1, 2].map(|id| CountDrop { id, counts, panic_once })
}
fn counts(counts: &[Cell<usize>; 3]) -> [usize; 3] { counts.each_ref().map(Cell::get) }

#[test]
fn vec_into_iter_drops_once_after_destructor_panics() {
    let drops = [const { Cell::new(0) }; 3];
    let panic_once = Cell::new(true);
    let v = heapless::Vec::<_, 3>::from_array(items(&drops, &panic_once));
    let mut iter = v.into_iter();
    drop(iter.next());
    assert!(catch_unwind(AssertUnwindSafe(|| drop(iter))).is_err());
    assert_eq!(counts(&drops), [1, 1, 1]);
}

#[test]
fn deque_clear_drops_once_after_destructor_panics() {
    let drops = [const { Cell::new(0) }; 3];
    let panic_once = Cell::new(true);
    let mut d = heapless::Deque::<_, 3>::new();
    d.extend(items(&drops, &panic_once));
    assert!(catch_unwind(AssertUnwindSafe(|| d.clear())).is_err());
    drop(d);
    assert_eq!(counts(&drops), [1, 1, 1]);
}

#[test]
fn history_clear_drops_once_after_destructor_panics() {
    let drops = [const { Cell::new(0) }; 3];
    let panic_once = Cell::new(true);
    let mut h = heapless::HistoryBuf::<_, 3>::new();
    h.extend(items(&drops, &panic_once));
    assert!(catch_unwind(AssertUnwindSafe(|| h.clear())).is_err());
    drop(h);
    assert_eq!(counts(&drops), [1, 1, 1]);
}
