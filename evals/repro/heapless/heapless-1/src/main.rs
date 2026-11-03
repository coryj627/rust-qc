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

use heapless::{index_map::FnvIndexMap, Deque, HistoryBuf};

fn main() {
    let mode = std::env::args().nth(1).expect("mode");
    match mode.as_str() {
        "deque" => {
            let mut q: Deque<D, 4> = Deque::new();
            assert!(q.push_back(D::new(1)).is_ok());
            assert!(q.push_back(D::new(2)).is_ok());
            arm();
            let r = catch_unwind(AssertUnwindSafe(|| q.clear()));
            println!("clear panicked: {}, len after clear: {}", r.is_err(), q.len());
            drop(q); // Deque::drop drops the same elements again
            println!("no UB: deque dropped cleanly");
        }
        "history_buf" => {
            let mut h: HistoryBuf<D, 4> = HistoryBuf::new();
            h.write(D::new(1));
            h.write(D::new(2));
            arm();
            let r = catch_unwind(AssertUnwindSafe(|| h.clear()));
            println!("clear panicked: {}, len after clear: {}", r.is_err(), h.len());
            drop(h);
            println!("no UB: history buf dropped cleanly");
        }
        "index_map" => {
            // Boxed keys, so that reading a key through a stale index touches freed memory.
            let mut m: FnvIndexMap<Box<u32>, D, 8> = FnvIndexMap::new();
            assert!(m.insert(Box::new(1), D::new(1)).is_ok());
            assert!(m.insert(Box::new(2), D::new(2)).is_ok());
            arm();
            let r = catch_unwind(AssertUnwindSafe(|| m.clear()));
            println!("clear panicked: {}, len after clear: {}", r.is_err(), m.len());
            // The entries are gone but the index slots were not reset. Make the map non-empty
            // so that lookups go ahead, then look up a key that only a stale slot refers to.
            assert!(m.insert(Box::new(3), D::new(3)).is_ok());
            println!("lookup of removed key: {}", m.get(&Box::new(2)).is_some());
            println!("no UB: index map survived");
        }
        _ => panic!("bad mode"),
    }
}
