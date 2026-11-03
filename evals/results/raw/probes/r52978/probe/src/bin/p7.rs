use moka::sync::Cache;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
fn main() {
    let c: Cache<u64, u64> = Cache::builder().max_capacity(1 << 40).build();
    let round = Arc::new(AtomicU64::new(0));
    let arrivals = Arc::new(AtomicU64::new(0));
    let n = 300_000u64;
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
    for _ in 0..5 { c.run_pending_tasks(); }
    println!("[inflate] no removes, weight 1 each: entry_count={} weighted_size={} real={}", c.entry_count(), c.weighted_size(), c.iter().count());
    // Timer wheel eventually evicts
    use moka::Expiry; use std::time::Instant;
    struct E; impl Expiry<u32,u32> for E { fn expire_after_create(&self,_:&u32,_:&u32,_:Instant)->Option<Duration>{Some(Duration::from_millis(100))} }
    let c: Cache<u32,u32> = Cache::builder().expire_after(E).build();
    for k in 0..100 { c.insert(k,k); }
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(2300));
    c.run_pending_tasks();
    println!("[timer] 2.3s after 100ms expiry: entry_count={}", c.entry_count());
    std::process::exit(0);
}
