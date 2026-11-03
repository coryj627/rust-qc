#[cfg(test)]
mod tests {
    use std::panic::{catch_unwind, AssertUnwindSafe};
    struct Bomb { _data: Box<u8>, armed: bool }
    impl Bomb { fn new(armed: bool) -> Self { Self { _data: Box::new(42), armed } } }
    impl Drop for Bomb {
        fn drop(&mut self) { if self.armed { self.armed = false; panic!("intentional destructor panic"); } }
    }
    #[test]
    fn vec_into_iter_panic() {
        let v = heapless::Vec::<_, 2>::from_array([Bomb::new(true)]);
        let result = catch_unwind(AssertUnwindSafe(|| drop(v.into_iter())));
        assert!(result.is_err());
    }
    #[test]
    fn vec_into_iter_consumed_box() {
        let v = heapless::Vec::<_, 2>::from_array([Box::new(42), Box::new(43)]);
        let mut it = v.into_iter();
        drop(it.next());
        drop(it);
    }
    #[test]
    fn history_clear_panic() {
        let mut h = heapless::HistoryBuf::<_, 2>::new();
        h.write(Bomb::new(true));
        assert!(catch_unwind(AssertUnwindSafe(|| h.clear())).is_err());
        drop(h);
    }
    #[test]
    fn history_write_panic() {
        let mut h = heapless::HistoryBuf::<_, 1>::new();
        h.write(Bomb::new(true));
        assert!(catch_unwind(AssertUnwindSafe(|| h.write(Bomb::new(false)))).is_err());
        drop(h);
    }
    #[test]
    fn cstring_borrow_hash() {
        use std::hash::{Hash, Hasher};
        let s = heapless::CString::<8>::from_bytes_with_nul(b"hello\0").unwrap();
        let mut a = std::collections::hash_map::DefaultHasher::new();
        let mut b = std::collections::hash_map::DefaultHasher::new();
        s.hash(&mut a);
        s.as_c_str().hash(&mut b);
        assert_eq!(a.finish(), b.finish());
    }
    #[test]
    fn cstring_requires_nul() {
        assert!(heapless::CString::<8>::from_bytes_with_nul(b"hello").is_err());
    }
    #[test]
    fn history_zero() {
        let h = heapless::HistoryBuf::<u8, 0>::new_with(42);
        assert_eq!(h.recent(), None);
    }
    #[test]
    fn deque_clear_panic() {
        let mut d = heapless::Deque::<_, 2>::new();
        assert!(d.push_back(Bomb::new(true)).is_ok());
        assert!(catch_unwind(AssertUnwindSafe(|| d.clear())).is_err());
        drop(d);
    }
    #[test]
    fn deque_full_make_contiguous() {
        let mut d = heapless::Deque::<_, 4>::new();
        for x in 0..4 { d.push_back(x).unwrap(); }
        for x in 4..7 { d.pop_front(); d.push_back(x).unwrap(); }
        assert_eq!(d.make_contiguous(), &[3, 4, 5, 6]);
        assert_eq!(d.pop_front(), Some(3));
        d.push_back(7).unwrap();
        assert_eq!(d.iter().copied().collect::<std::vec::Vec<_>>(), [4, 5, 6, 7]);
    }
    #[test]
    fn bytes_fill_capacity() {
        use bytes::BufMut;
        let mut v = heapless::Vec::<u8, 4>::new();
        v.put_slice(b"abcd");
        assert_eq!(&v[..], b"abcd");
    }
    #[test]
    fn indexmap_retain_panic() {
        let mut m = heapless::index_map::FnvIndexMap::<u32, Box<u32>, 4>::new();
        m.insert(10, Box::new(10)).unwrap();
        m.insert(20, Box::new(20)).unwrap();
        let mut n = 0;
        assert!(catch_unwind(AssertUnwindSafe(|| m.retain(|_, _| {
            n += 1;
            if n == 2 { panic!("predicate panic"); }
            false
        }))).is_err());
        assert_eq!(m.get(&20).map(|v| **v), Some(20));
    }
    #[test]
    fn indexmap_clear_panic() {
        let mut m = heapless::index_map::FnvIndexMap::<u32, Bomb, 4>::new();
        assert!(m.insert(10, Bomb::new(true)).is_ok());
        assert!(catch_unwind(AssertUnwindSafe(|| m.clear())).is_err());
        let _ = m.entry(10).or_insert(Bomb::new(false));
    }
    #[derive(Default)]
    struct Identity(u64);
    impl std::hash::Hasher for Identity {
        fn finish(&self) -> u64 { self.0 }
        fn write(&mut self, bytes: &[u8]) {
            self.0 = 0;
            for (i, b) in bytes.iter().take(8).enumerate() { self.0 |= u64::from(*b) << (8 * i); }
        }
        fn write_u32(&mut self, n: u32) { self.0 = u64::from(n); }
    }
    type IdentityMap<V, const N: usize> = heapless::IndexMap<u32, V, hash32::BuildHasherDefault<Identity>, N>;
    #[test]
    fn indexmap_truncate_collisions() {
        let mut m = IdentityMap::<u8, 4>::new();
        m.insert(0, 1).unwrap();
        m.insert(4, 2).unwrap();
        m.insert(8, 3).unwrap();
        m.remove(&0);
        m.truncate(1);
        assert_eq!(m.get(&8), Some(&3));
    }
    #[test]
    fn indexmap_large_capacity() {
        let mut m = IdentityMap::<u8, 65536>::new();
        for k in 0..65536 { m.insert(k, 42).unwrap(); }
        assert_eq!(m.len(), 65536);
    }
    #[test]
    fn spsc_peek_threads() {
        let mut q = heapless::spsc::Queue::<u64, 2>::new();
        let (mut p, c) = q.split();
        std::thread::scope(|scope| {
            scope.spawn(move || { p.enqueue(42).unwrap(); });
            loop {
                if let Some(v) = c.peek() { assert_eq!(*v, 42); break; }
                std::thread::yield_now();
            }
        });
    }
    #[test]
    fn sorted_mut_pop() {
        use heapless::sorted_linked_list::{SortedLinkedList, Max};
        let mut l = SortedLinkedList::<Box<i32>, Max, 4, u8>::new_u8();
        for x in [1, 2, 3] { l.push(Box::new(x)).unwrap(); }
        let mut guard = l.find_mut(|v| **v == 2).unwrap();
        **guard = 4;
        let value = guard.pop();
        assert_eq!(*value, 4);
        drop(value);
        while let Some(v) = l.pop() { println!("{v}"); }
    }
    #[test]
    fn vec_retain_panic() {
        let mut v = heapless::Vec::<_, 3>::from_array([1, 2, 3]);
        let mut n = 0;
        assert!(catch_unwind(AssertUnwindSafe(|| v.retain(|_| {
            n += 1;
            if n == 2 { panic!("predicate panic"); }
            false
        }))).is_err());
        assert_eq!(&v[..], &[2, 3]);
    }
    #[test]
    fn pool_concurrent_reuse() {
        heapless::box_pool!(RacePool: u64);
        RacePool.manage(Box::leak(Box::new(heapless::pool::boxed::BoxBlock::new())));
        std::thread::scope(|scope| {
            for _ in 0..2 {
                scope.spawn(|| {
                    for i in 0..100 {
                        if let Ok(b) = RacePool.alloc(i) { assert_eq!(*b, i); }
                    }
                });
            }
        });
    }
    #[test]
    fn indexmap_retain_stale_index() {
        let mut m = heapless::index_map::FnvIndexMap::<u32, Box<u32>, 4>::new();
        m.insert(10, Box::new(10)).unwrap();
        m.insert(20, Box::new(20)).unwrap();
        assert!(catch_unwind(AssertUnwindSafe(|| m.retain(|k, _| {
            if *k == 10 { true } else { panic!("predicate panic before dropping anything"); }
        }))).is_err());
        assert_eq!(m.len(), 2);
    }
    #[test]
    fn sorted_mut_pop_plain() {
        use heapless::sorted_linked_list::{SortedLinkedList, Max};
        let mut l = SortedLinkedList::<i32, Max, 4, u8>::new_u8();
        for x in [1, 2, 3] { l.push(x).unwrap(); }
        let mut guard = l.find_mut(|v| *v == 2).unwrap();
        *guard = 4;
        assert_eq!(guard.pop(), 4);
        assert_eq!(l.iter().copied().collect::<std::vec::Vec<_>>(), [3, 1]);
    }
    #[test]
    fn vec_extend_zst_overflow() {
        let input = [(); usize::MAX];
        let mut v = heapless::Vec::<(), 2>::new();
        v.push(()).unwrap();
        assert!(v.extend_from_slice(&input).is_err());
    }
}
