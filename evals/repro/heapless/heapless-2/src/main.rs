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

use heapless::HistoryBuf;

fn main() {
    let mut h: HistoryBuf<D, 2> = HistoryBuf::new();
    h.write(D::new(1));
    h.write(D::new(2)); // now full, the next write overwrites the slot holding 1
    arm();
    let r = catch_unwind(AssertUnwindSafe(|| h.write(D::new(3))));
    println!("write panicked: {}, len: {}", r.is_err(), h.len());
    drop(h); // drops the slot whose destructor already ran
    println!("no UB: history buf dropped cleanly");
}
