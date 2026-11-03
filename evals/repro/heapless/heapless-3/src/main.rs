use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};

static ARMED: AtomicBool = AtomicBool::new(false);

/// Owns heap memory so that a second drop is a double free Miri can see.
/// The destructor panics once, when armed.
struct D(#[allow(dead_code)] Box<u32>);
impl D {
    fn new(v: u32) -> Self { D(Box::new(v)) }
}
impl Drop for D {
    fn drop(&mut self) {
        if ARMED.swap(false, Ordering::SeqCst) {
            panic!("panicking Drop");
        }
    }
}
fn arm() { ARMED.store(true, Ordering::SeqCst); }

use heapless::Vec;

fn main() {
    let mut v: Vec<D, 3> = Vec::new();
    assert!(v.push(D::new(1)).is_ok());
    assert!(v.push(D::new(2)).is_ok());
    assert!(v.push(D::new(3)).is_ok());
    let mut it = v.into_iter();
    let first = it.next().unwrap(); // moved out; still owned here
    drop(first);
    arm();
    // IntoIter::drop drops the tail and panics before it sets len to 0, so the inner Vec's
    // own Drop runs during unwinding over all 3 elements, including the one moved out above.
    let r = catch_unwind(AssertUnwindSafe(move || drop(it)));
    println!("no UB: IntoIter drop returned, panicked: {}", r.is_err());
}
