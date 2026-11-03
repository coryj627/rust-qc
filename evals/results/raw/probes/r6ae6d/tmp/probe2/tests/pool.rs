use heapless::{box_pool, pool::boxed::BoxBlock};
box_pool!(P: [u64; 4]);
#[test]
fn concurrent_alloc_drop() {
    for _ in 0..4 { let b: &'static mut BoxBlock<[u64; 4]> = Box::leak(Box::new(BoxBlock::new())); P.manage(b); }
    let hs: std::vec::Vec<_> = (0..3u64).map(|t| std::thread::spawn(move || {
        for i in 0..200u64 { if let Ok(mut b) = P.alloc([t, i, 0, 0]) { b[2] = i; std::thread::yield_now(); drop(b); } }
    })).collect();
    for h in hs { h.join().unwrap(); }
}
