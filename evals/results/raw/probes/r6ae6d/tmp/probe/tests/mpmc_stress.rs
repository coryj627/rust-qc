use heapless::mpmc::Queue;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::*};
static Q: Queue<u64, 2> = Queue::new();
static DONE: AtomicBool = AtomicBool::new(false);
static NEXT: AtomicU64 = AtomicU64::new(1);
#[test]
fn stress() {
    let secs: u64 = 45;
    let start = std::time::Instant::now();
    let mut hs = vec![];
    for _ in 0..8 { hs.push(std::thread::spawn(|| { while !DONE.load(Relaxed) {
        let id = NEXT.fetch_add(1, Relaxed);
        let mut v = id; loop { match Q.enqueue(v) { Ok(()) => break, Err(x) => { v = x; if DONE.load(Relaxed) { return; } std::thread::yield_now(); } } }
    } })); }
    let mut cs = vec![];
    for _ in 0..8 { cs.push(std::thread::spawn(|| { let mut got = std::vec::Vec::new(); while !DONE.load(Relaxed) { if let Some(x) = Q.dequeue() { got.push(x); } else { std::thread::yield_now(); } } got })); }
    while start.elapsed().as_secs() < secs { std::thread::sleep(std::time::Duration::from_millis(200)); }
    DONE.store(true, Relaxed);
    for h in hs { h.join().unwrap(); }
    let mut all = std::vec::Vec::new();
    for c in cs { all.extend(c.join().unwrap()); }
    let n = all.len(); all.sort_unstable();
    let mut dups = 0; let mut ex = vec![];
    for w in all.windows(2) { if w[0] == w[1] { dups += 1; if ex.len() < 5 { ex.push(w[0]); } } }
    println!("dequeued {n} values, duplicates {dups}, examples {ex:?}");
    assert_eq!(dups, 0);
}
