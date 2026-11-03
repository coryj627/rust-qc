use heapless::Vec;
use std::panic::{catch_unwind, AssertUnwindSafe};

fn main() {
    let mut v = Vec::<u64, 6>::new();
    for x in [10u64, 20, 30, 40, 50, 60] {
        v.push(x).unwrap();
    }
    let r = catch_unwind(AssertUnwindSafe(|| {
        v.retain(|k| match *k {
            10 => true,
            20 => false,
            _ => panic!("user predicate panic"),
        })
    }));
    println!("no UB: retain panicked: {}, v={:?}", r.is_err(), v);
}
