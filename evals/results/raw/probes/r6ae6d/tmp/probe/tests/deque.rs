use heapless::Deque;

// Reach front=7, back=3, len=8 in a Deque<_,12>: FGH....ABCDE
fn wrapped_second_branch() -> Deque<u32, 12> {
    let mut d: Deque<u32, 12> = Deque::new();
    for i in 0..7 { d.push_back(1000 + i).unwrap(); }
    for _ in 0..7 { d.pop_front().unwrap(); }
    // front = back = 7
    for i in 0..8 { d.push_back(i).unwrap(); }
    d
}

#[test]
fn second_branch_len() {
    let mut d = wrapped_second_branch();
    assert_eq!(d.len(), 8);
    let before: std::vec::Vec<u32> = d.iter().copied().collect();
    println!("before {:?}", before);
    let s: std::vec::Vec<u32> = d.make_contiguous().to_vec();
    println!("returned slice {:?}", s);
    println!("len after {}", d.len());
    assert_eq!(d.len(), 8, "len changed by make_contiguous");
}

#[test]
fn second_branch_uninit_read() {
    let mut d = wrapped_second_branch();
    d.make_contiguous();
    // touches every "element" including the uninit one
    let total: u64 = d.iter().map(|x| *x as u64).sum();
    println!("{total}");
}

// Full deque, wrapped, front_len <= back_len : reach 4th branch with free == 0
#[test]
fn full_back_oob() {
    #[repr(C)]
    struct Guarded { d: Deque<u32, 4>, canary: [u32; 8] }
    let mut g = Guarded { d: Deque::new(), canary: [0xAAAA_AAAA; 8] };
    g.d.push_back(0).unwrap();
    g.d.push_back(1).unwrap();
    g.d.push_back(2).unwrap();
    g.d.pop_front();
    g.d.pop_front();
    g.d.pop_front();          // front=back=3
    for i in 0..4 { g.d.push_back(10 + i).unwrap(); } // full, front=3 ; front_len=1, back_len=3
    assert!(g.d.is_full());
    assert_eq!(g.d.make_contiguous(), &[10, 11, 12, 13]);
    g.d.pop_front();
    assert!(!g.d.is_full());
    for i in 0..3 {
        println!("push_back {:?} len {}", g.d.push_back(0xDEAD_0000 + i), g.d.len());
    }
    println!("canary {:x?}", g.canary);
    assert_eq!(g.canary, [0xAAAA_AAAA; 8]);
}

#[test]
fn clear_panic_double_drop() {
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
    static DROPS: AtomicUsize = AtomicUsize::new(0);
    struct P(bool);
    impl Drop for P { fn drop(&mut self) { DROPS.fetch_add(1, SeqCst); if self.0 { self.0 = false; panic!("boom") } } }
    {
        let mut d: Deque<P, 4> = Deque::new();
        d.push_back(P(true)).ok().unwrap();
        d.push_back(P(false)).ok().unwrap();
        let _ = catch_unwind(AssertUnwindSafe(|| d.clear()));
        println!("after clear: drops={} len={}", DROPS.load(SeqCst), d.len());
    }
    println!("total drops={} for 2 values", DROPS.load(SeqCst));
    assert_eq!(DROPS.load(SeqCst), 2);
}

#[test]
fn second_branch_box_double_free() {
    let mut d: Deque<std::boxed::Box<u32>, 12> = Deque::new();
    for i in 0..7 { d.push_back(Box::new(i)).unwrap(); }
    for _ in 0..7 { d.pop_front().unwrap(); }
    for i in 0..8 { d.push_back(Box::new(i)).unwrap(); }
    d.make_contiguous();
    println!("len {}", d.len());
}
