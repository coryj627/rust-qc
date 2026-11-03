use heapless::{CString, HistoryBuf, LinearMap};
use heapless::index_map::FnvIndexMap;

#[test]
fn linear_map_from_iter_dup() {
    let mut m: LinearMap<u8, char, 4> = [(1, 'a'), (1, 'b')].into_iter().collect();
    println!("len {} get {:?}", m.len(), m.get(&1));
    let removed = m.remove(&1);
    println!("removed {:?}, still get {:?}", removed, m.get(&1));
    assert_eq!(m.get(&1), None);
}

#[test]
fn cstring_from_bytes_with_nul_no_nul() {
    let r = CString::<8>::from_bytes_with_nul(b"abc");
    println!("{:?}", r);
    assert!(r.is_err());
}

#[test]
fn cstring_from_bytes_with_nul_empty() {
    let r = CString::<8>::from_bytes_with_nul(b"");
    println!("{:?}", r);
    assert!(r.is_err());
}

#[test]
fn indexmap_into_iter_order() {
    let mut m: FnvIndexMap<u8, u8, 8> = FnvIndexMap::new();
    m.insert(1, 1).unwrap(); m.insert(2, 2).unwrap(); m.insert(3, 3).unwrap();
    let a: std::vec::Vec<u8> = m.iter().map(|(k, _)| *k).collect();
    let b: std::vec::Vec<u8> = m.into_iter().map(|(k, _)| k).collect();
    println!("iter {a:?} into_iter {b:?}");
    assert_eq!(a, b);
}

#[test]
fn historybuf_new_with_zero() {
    let h: HistoryBuf<u8, 0> = HistoryBuf::new_with(1);
    println!("len {} full {}", h.len(), h.is_full());
    println!("recent_index {:?}", h.recent_index());
    println!("recent {:?}", h.recent());
}

#[test]
fn historybuf_clear_and_write_panic_double_drop() {
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
    static DROPS: AtomicUsize = AtomicUsize::new(0);
    struct P(bool);
    impl Drop for P { fn drop(&mut self) { DROPS.fetch_add(1, SeqCst); if self.0 { self.0 = false; panic!("boom") } } }
    {
        let mut h: HistoryBuf<P, 2> = HistoryBuf::new();
        h.write(P(true));
        h.write(P(false));
        let _ = catch_unwind(AssertUnwindSafe(|| h.write(P(false)))); // overwrites the panicky one
        println!("after write panic: drops={} len={}", DROPS.load(SeqCst), h.len());
    }
    println!("write: total drops={} for 3 values", DROPS.load(SeqCst));
    let w = DROPS.swap(0, SeqCst);
    {
        let mut h: HistoryBuf<P, 2> = HistoryBuf::new();
        h.write(P(true));
        h.write(P(false));
        let _ = catch_unwind(AssertUnwindSafe(|| h.clear()));
        println!("after clear panic: drops={} len={}", DROPS.load(SeqCst), h.len());
    }
    println!("clear: total drops={} for 2 values", DROPS.load(SeqCst));
    let c = DROPS.load(SeqCst);
    assert_eq!((w, c), (3, 2));
}
