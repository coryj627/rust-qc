use moka::sync::Cache;
use std::time::Duration;
fn survivors(c: &Cache<u32,u32>, n: u32) -> Vec<u32> { (0..n).filter(|k| c.contains_key(k)).collect() }
fn drain(c: &Cache<u32,u32>) { for _ in 0..200 { c.run_pending_tasks(); } }
fn main() {
    // A: one match-all predicate
    let c: Cache<u32,u32> = Cache::builder().max_capacity(100000).support_invalidation_closures().build();
    for k in 0..2000 { c.insert(k, k); }
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(5));
    c.invalidate_entries_if(|_, _| true).unwrap();
    c.run_pending_tasks();
    println!("after 1 run_pending_tasks: entry_count={}", c.entry_count());
    drain(&c);
    println!("single pred survivors: {:?} entry_count={}", survivors(&c, 2000), c.entry_count());

    // B: predicate registered during a scan; A retires part-way through the deque.
    let c: Cache<u32,u32> = Cache::builder().max_capacity(100000).support_invalidation_closures().build();
    for k in 0..1100 { c.insert(k, k); }
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(5));
    c.invalidate_entries_if(|_, _| false).unwrap();
    c.run_pending_tasks(); // scan 1
    std::thread::sleep(Duration::from_millis(5));
    for k in 1100..1160 { c.insert(k, k); }
    c.run_pending_tasks(); // scan 2
    for k in 1160..1220 { c.insert(k, k); }
    std::thread::sleep(Duration::from_millis(5));
    c.invalidate_entries_if(|k, _| *k < 1100).unwrap();
    drain(&c);
    let s = survivors(&c, 1100);
    println!("second pred (A retires mid-queue): {} of 1100 survive; first few {:?}", s.len(), &s[..s.len().min(8)]);
    // B2: A's scan ends at the queue end while B waits.
    let c: Cache<u32,u32> = Cache::builder().max_capacity(100000).support_invalidation_closures().build();
    for k in 0..300 { c.insert(k, k); }
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(5));
    c.invalidate_entries_if(|_, _| false).unwrap();
    c.invalidate_entries_if(|_, _| false).unwrap();
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(5));
    println!("(no-op)");
    let c: Cache<u32,u32> = Cache::builder().max_capacity(100000).support_invalidation_closures().build();
    for k in 0..800 { c.insert(k, k); }
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(5));
    c.invalidate_entries_if(|_, _| false).unwrap();
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(5));
    c.invalidate_entries_if(|_, _| true).unwrap();
    drain(&c);
    println!("second pred match-all, first ends at queue end: survivors {:?}", survivors(&c, 800));
    // control B alone
    let c: Cache<u32,u32> = Cache::builder().max_capacity(100000).support_invalidation_closures().build();
    for k in 0..1000 { c.insert(k, k); }
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(5));
    c.invalidate_entries_if(|k, _| *k < 1000).unwrap();
    drain(&c);
    println!("control second pred alone: {:?}", survivors(&c, 1000));
}
