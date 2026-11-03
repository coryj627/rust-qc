use heapless::Vec;
use std::panic::{catch_unwind, AssertUnwindSafe};
#[test]
fn retain_panic_after_delete() {
    let mut v: Vec<u32, 8> = Vec::from_slice(&[1, 2, 3, 4]).unwrap();
    let mut n = 0;
    let _ = catch_unwind(AssertUnwindSafe(|| v.retain(|_| { n += 1; if n == 2 { panic!("boom") } false })));
    println!("{:?}", v);
    assert_eq!(v, [2, 3, 4]);
}
