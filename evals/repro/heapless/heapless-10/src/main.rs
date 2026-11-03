use critical_section as _;
use heapless::{box_pool, pool::boxed::BoxBlock};

box_pool!(P: u128);

fn main() {
    let block: &'static mut BoxBlock<u128> = unsafe {
        static mut BLOCK: BoxBlock<u128> = BoxBlock::new();
        (&raw mut BLOCK).as_mut().unwrap()
    };
    P.manage(block); // links a node whose `next` field was never initialised
    let b = P.alloc(7).ok().unwrap();
    assert_eq!(*b, 7);
    drop(b);
    println!("no UB");
}
