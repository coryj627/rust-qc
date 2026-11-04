use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

#[test]
fn vec_iterator_drops_each_element_once_after_drop_panics() {
    static COUNTS: [AtomicUsize; 3] = [const { AtomicUsize::new(0) }; 3];
    static PANICKED: AtomicBool = AtomicBool::new(false);
    #[derive(Debug)]
    struct Item(usize);
    impl Drop for Item {
        fn drop(&mut self) {
            COUNTS[self.0].fetch_add(1, Ordering::Relaxed);
            if self.0 == 1 && !PANICKED.swap(true, Ordering::Relaxed) {
                panic!("element drop failed once");
            }
        }
    }
    let mut iter = heapless::Vec::<Item, 3>::from_array([Item(0), Item(1), Item(2)]).into_iter();
    let first = iter.next().unwrap();
    assert!(catch_unwind(AssertUnwindSafe(|| drop(iter))).is_err());
    drop(first);
    assert_eq!(
        COUNTS.each_ref().map(|n| n.load(Ordering::Relaxed)),
        [1, 1, 1]
    );
}

#[test]
fn deque_clear_drops_each_element_once_after_drop_panics() {
    static COUNTS: [AtomicUsize; 2] = [const { AtomicUsize::new(0) }; 2];
    static PANICKED: AtomicBool = AtomicBool::new(false);
    #[derive(Debug)]
    struct Item(usize);
    impl Drop for Item {
        fn drop(&mut self) {
            COUNTS[self.0].fetch_add(1, Ordering::Relaxed);
            if self.0 == 0 && !PANICKED.swap(true, Ordering::Relaxed) {
                panic!("element drop failed once");
            }
        }
    }
    let mut q = heapless::Deque::<Item, 2>::new();
    q.push_back(Item(0)).unwrap();
    q.push_back(Item(1)).unwrap();
    assert!(catch_unwind(AssertUnwindSafe(|| q.clear())).is_err());
    drop(q);
    assert_eq!(COUNTS.each_ref().map(|n| n.load(Ordering::Relaxed)), [1, 1]);
}

#[test]
fn history_clear_drops_each_element_once_after_drop_panics() {
    static COUNTS: [AtomicUsize; 2] = [const { AtomicUsize::new(0) }; 2];
    static PANICKED: AtomicBool = AtomicBool::new(false);
    #[derive(Debug)]
    struct Item(usize);
    impl Drop for Item {
        fn drop(&mut self) {
            COUNTS[self.0].fetch_add(1, Ordering::Relaxed);
            if self.0 == 0 && !PANICKED.swap(true, Ordering::Relaxed) {
                panic!("element drop failed once");
            }
        }
    }
    let mut q = heapless::HistoryBuf::<Item, 2>::new();
    q.write(Item(0));
    q.write(Item(1));
    assert!(catch_unwind(AssertUnwindSafe(|| q.clear())).is_err());
    drop(q);
    assert_eq!(COUNTS.each_ref().map(|n| n.load(Ordering::Relaxed)), [1, 1]);
}
