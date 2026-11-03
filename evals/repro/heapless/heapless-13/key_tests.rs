
// Appended to a scratch copy of src/spsc.rs. Sets the private indices directly, since on a
// 64-bit host the public API needs about 2^63 dequeues to get head this high.
#[cfg(test)]
mod key_tests {
    use super::*;
    use core::cell::UnsafeCell;
    use core::mem::MaybeUninit;

    #[test]
    fn iter_index_overflow() {
        let q = Queue::<(), { usize::MAX - 1 }> {
            head: AtomicUsize::new(usize::MAX - 3),
            tail: AtomicUsize::new(3),
            buffer: [const { UnsafeCell::new(MaybeUninit::new(())) }; usize::MAX - 1],
        };
        assert_eq!(q.iter().count(), 5);
    }
}
