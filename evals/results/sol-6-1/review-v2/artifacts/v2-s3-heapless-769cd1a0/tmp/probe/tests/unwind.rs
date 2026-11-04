use std::{cell::Cell, panic::{catch_unwind, AssertUnwindSafe}};
use heapless::{Deque, HistoryBuf, Vec};

#[derive(Debug)]
struct Item<'a> { id: usize, counts: &'a [Cell<usize>] }
impl Drop for Item<'_> {
    fn drop(&mut self) {
        let prior = self.counts[self.id].get();
        self.counts[self.id].set(prior + 1);
        if self.id == 1 && prior == 0 { panic!("one destructor failure"); }
    }
}

#[test]
fn deque_clear_does_not_drop_elements_twice_after_unwind() {
    let counts = [Cell::new(0), Cell::new(0)];
    let mut q = Deque::<_, 2>::new();
    q.push_back(Item { id: 0, counts: &counts }).unwrap();
    q.push_back(Item { id: 1, counts: &counts }).unwrap();
    assert!(catch_unwind(AssertUnwindSafe(|| q.clear())).is_err());
    drop(q);
    assert_eq!([counts[0].get(), counts[1].get()], [1, 1]);
}

#[test]
fn history_clear_does_not_drop_elements_twice_after_unwind() {
    let counts = [Cell::new(0), Cell::new(0)];
    let mut q = HistoryBuf::<_, 2>::new();
    q.write(Item { id: 0, counts: &counts });
    q.write(Item { id: 1, counts: &counts });
    assert!(catch_unwind(AssertUnwindSafe(|| q.clear())).is_err());
    drop(q);
    assert_eq!([counts[0].get(), counts[1].get()], [1, 1]);
}

#[test]
fn vec_iterator_does_not_drop_already_yielded_elements_after_unwind() {
    let counts = [Cell::new(0), Cell::new(0)];
    let q = Vec::<_, 2>::from_array([
        Item { id: 0, counts: &counts },
        Item { id: 1, counts: &counts },
    ]);
    let mut it = q.into_iter();
    let first = it.next().unwrap();
    assert!(catch_unwind(AssertUnwindSafe(|| drop(it))).is_err());
    assert_eq!(counts[0].get(), 0, "yielded element is still owned by the caller");
    drop(first);
}

#[test]
fn index_map_retain_keeps_lookup_consistent_after_unwind() {
    use heapless::index_map::FnvIndexMap;
    let mut m = FnvIndexMap::<_, _, 4>::new();
    for i in 0..3 { m.insert(i, i + 10).unwrap(); }
    assert!(catch_unwind(AssertUnwindSafe(|| m.retain(|k, _| {
        if *k == 1 { panic!("predicate failure"); }
        *k != 0
    }))).is_err());
    for (key, value) in m.iter() {
        assert_eq!(m.get(key), Some(value), "retained key {key}");
    }
}
