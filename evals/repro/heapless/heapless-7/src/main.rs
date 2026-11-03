use critical_section as _;
use heapless::{box_pool, pool::boxed::BoxBlock};

box_pool!(P: u128);

fn main() {
    let block: &'static mut BoxBlock<u128> = unsafe {
        static mut BLOCK: BoxBlock<u128> = BoxBlock::new();
        (&raw mut BLOCK).as_mut().unwrap()
    };
    let base = block as *mut BoxBlock<u128> as usize;
    P.manage(block);
    let b = P.alloc(7).ok().unwrap();
    let data = &*b as *const u128 as usize;
    println!("size_of BoxBlock<u128> = {}", size_of::<BoxBlock<u128>>());
    println!("data offset inside the node = {}", data - base);
    assert_eq!(*b, 7);
    std::process::exit(if data != base { 0 } else { 1 });
}
