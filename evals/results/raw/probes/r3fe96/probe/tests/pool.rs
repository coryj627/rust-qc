#![cfg(feature = "pool")]
use core::ptr::addr_of_mut;
use heapless::box_pool;
use heapless::pool::boxed::BoxBlock;
box_pool!(P: [u64; 2]);
#[test]
fn pool_race() {
    for b in unsafe {
        static mut BLOCKS: [BoxBlock<[u64; 2]>; 3] = [const { BoxBlock::new() }; 3];
        addr_of_mut!(BLOCKS).as_mut().unwrap()
    } {
        P.manage(b);
    }
    std::thread::scope(|s| {
        for t in 0..3u64 {
            s.spawn(move || {
                for i in 0..30u64 {
                    if let Ok(mut b) = P.alloc([t, i]) {
                        std::thread::yield_now();
                        b[0] = t + 1;
                        assert_eq!(b[1], i);
                    }
                    std::thread::yield_now();
                }
            });
        }
    });
}
