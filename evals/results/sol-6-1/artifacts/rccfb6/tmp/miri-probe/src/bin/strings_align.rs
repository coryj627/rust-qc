use std::alloc::{GlobalAlloc, Layout, System};
use simd_json::prelude::*;
struct Aligned;
unsafe impl GlobalAlloc for Aligned {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        unsafe { System.alloc(Layout::from_size_align(l.size(), l.align().max(16)).unwrap()) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, Layout::from_size_align(l.size(), l.align().max(16)).unwrap()) }
    }
}
#[global_allocator]
static ALIGNED: Aligned = Aligned;
fn main() {
    let mut input = br#"["a","b\nc"]"#.to_vec();
    let tape = simd_json::to_tape(&mut input).unwrap();
    println!("{}", tape.as_value().encode());
}
