// Timer node freed with the pointer left behind in DeqNodes.
use moka::{sync::Cache, Expiry};
use std::{sync::{Arc, Barrier, atomic::{AtomicBool, Ordering}}, time::{Duration, Instant}};
struct E;
impl Expiry<u32, u32> for E {
    fn expire_after_create(&self, _: &u32, _: &u32, _: Instant) -> Option<Duration> { Some(Duration::from_millis(1500)) }
    fn expire_after_read(&self, _: &u32, _: &u32, _: Instant, _: Option<Duration>, _: Instant) -> Option<Duration> { None }
}
fn main() {
    let block = Arc::new(AtomicBool::new(true));
    let b1 = Arc::new(Barrier::new(2));
    let b2 = Arc::new(Barrier::new(2));
    let (bl, b1l, b2l) = (block.clone(), b1.clone(), b2.clone());
    let c: Cache<u32, u32> = Cache::builder()
        .max_capacity(10)
        .weigher(|_, v: &u32| *v)
        .expire_after(E)
        .eviction_listener(move |_k, _v, _c| {
            if bl.swap(false, Ordering::SeqCst) {
                b1l.wait(); // tell reader we are inside the maintenance pass
                b2l.wait(); // wait until reader changed the expiry and time passed
            }
        })
        .build();
    c.insert(1, 1);
    c.run_pending_tasks();
    c.insert(2, 100); // over capacity: rejected with a listener call
    let c2 = c.clone();
    let (b1r, b2r) = (b1.clone(), b2.clone());
    let reader = std::thread::spawn(move || {
        b1r.wait();
        println!("reader get = {:?}", c2.get(&1)); // expiry -> None; ReadOp queued
        std::thread::sleep(Duration::from_millis(3500));
        b2r.wait();
    });
    c.run_pending_tasks();
    reader.join().unwrap();
    println!("after pass 1; now apply the queued read op");
    c.run_pending_tasks();
    println!("after pass 2; insert and churn");
    for i in 0..50 { c.insert(1, 1 + (i % 3)); c.run_pending_tasks(); }
    c.invalidate(&1);
    c.run_pending_tasks();
    println!("done count={}", c.entry_count());
}
