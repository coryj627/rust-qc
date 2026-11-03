use moka::policy::EvictionPolicy;
use moka::sync::Cache;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn herd(c: &Cache<u64, u64>, n: u64) {
    let round = Arc::new(AtomicU64::new(0));
    let arrivals = Arc::new(AtomicU64::new(0));
    let mut hs = vec![];
    for t in 0..4u64 {
        let (c, round, arrivals) = (c.clone(), round.clone(), arrivals.clone());
        hs.push(std::thread::spawn(move || {
            for k in 0..n {
                while round.load(Ordering::Acquire) != k { std::hint::spin_loop(); }
                c.insert(k, t);
                if arrivals.fetch_add(1, Ordering::AcqRel) % 4 == 3 { round.store(k + 1, Ordering::Release); }
            }
        }));
    }
    for h in hs { h.join().unwrap(); }
}
fn main() {
    for (name, p) in [("lru", EvictionPolicy::lru()), ("tinylfu", EvictionPolicy::default())] {
        let c: Cache<u64, u64> = Cache::builder().max_capacity(5000).eviction_policy(p).build();
        herd(&c, 200_000);
        for _ in 0..5 { c.run_pending_tasks(); }
        println!("[{name}] cap 5000 after herd inserts: entry_count={} weighted={} real={}", c.entry_count(), c.weighted_size(), c.iter().count());
    }
    // Key-lock orphan: listener sees interleaving? count concurrency per key
    // Lost wakeups / blocking removal notification ordering.
    let order = Arc::new(Mutex::new(Vec::new()));
    let o2 = order.clone();
    let c: Cache<u32, u32> = Cache::builder().max_capacity(100).eviction_listener(move |k, v, cause| { o2.lock().unwrap().push((*k, v, cause)); std::thread::sleep(Duration::from_micros(1)); }).build();
    let mut hs = vec![];
    for t in 0..4u32 {
        let c = c.clone();
        hs.push(std::thread::spawn(move || { for i in 0..20000u32 { c.insert(i % 2, t << 24 | i); } }));
    }
    for h in hs { h.join().unwrap(); }
    println!("[order] notifications {}", order.lock().unwrap().len());
    std::process::exit(0);
}
