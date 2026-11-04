use heapless::{HistoryBuf, Vec};
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn resize_default_initializes_each_new_element() {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    #[derive(Clone, Debug, PartialEq)]
    struct Value(usize);
    impl Default for Value {
        fn default() -> Self { Self(NEXT.fetch_add(1, Ordering::Relaxed)) }
    }
    NEXT.store(0, Ordering::Relaxed);
    let mut v = Vec::<Value, 3>::new();
    v.resize_default(3).unwrap();
    assert_eq!(v.as_slice(), [Value(0), Value(1), Value(2)]);
}

#[test]
fn zero_capacity_history_is_empty() {
    let b = HistoryBuf::<u8, 0>::new_with(42);
    assert!(b.is_empty());
    assert_eq!(b.oldest(), None);
    assert_eq!(b.recent(), None);
}
