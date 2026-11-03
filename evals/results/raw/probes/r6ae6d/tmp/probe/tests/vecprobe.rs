use heapless::Vec;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};

#[test]
fn into_iter_drop_panic_double_drop() {
    static DROPS: AtomicUsize = AtomicUsize::new(0);
    struct P(bool, std::boxed::Box<u32>);
    impl Drop for P { fn drop(&mut self) { DROPS.fetch_add(1, SeqCst); if self.0 { self.0 = false; panic!("boom") } } }
    let mut v: Vec<P, 4> = Vec::new();
    v.push(P(false, Box::new(0))).ok().unwrap();
    v.push(P(true, Box::new(1))).ok().unwrap();
    v.push(P(false, Box::new(2))).ok().unwrap();
    let r = catch_unwind(AssertUnwindSafe(move || {
        let mut it = v.into_iter();
        let first = it.next().unwrap();
        drop(first);
        drop(it);
    }));
    println!("panicked={} drops={} for 3 values", r.is_err(), DROPS.load(SeqCst));
    assert_eq!(DROPS.load(SeqCst), 3);
}

#[test]
fn bufmut_fill_exact() {
    use bytes::BufMut;
    let mut v: Vec<u8, 8> = Vec::new();
    assert_eq!(v.remaining_mut(), 8);
    v.put_slice(&[1; 8]);
    assert_eq!(v.len(), 8);
}

