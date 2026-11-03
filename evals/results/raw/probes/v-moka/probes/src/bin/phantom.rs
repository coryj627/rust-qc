use moka::sync::Cache;
use moka::policy::EvictionPolicy;
use std::thread;
fn churn(c: &Cache<u32, u32>, secs: u64, keys: u32) {
    let end = std::time::Instant::now() + std::time::Duration::from_secs(secs);
    let mut hs = vec![];
    for t in 0..4u32 {
        let c = c.clone();
        hs.push(thread::spawn(move || {
            let mut i = t * 2654435761 + 1;
            while std::time::Instant::now() < end {
                i ^= i << 13; i ^= i >> 17; i ^= i << 5;
                let k = (i >> 8) % keys;
                if i & 1 == 0 { c.insert(k, i); } else { c.invalidate(&k); }
            }
        }));
    }
    for h in hs { h.join().unwrap(); }
    for k in 0..keys { c.invalidate(&k); }
    for _ in 0..5 { c.run_pending_tasks(); }
}
fn main() {
    let c: Cache<u32, u32> = Cache::new(1000);
    churn(&c, 5, 4);
    println!("tinylfu cap1000: entry_count={} weighted={} real={}", c.entry_count(), c.weighted_size(), c.iter().count());
    for k in 10000..10500 { c.insert(k, k); }
    for _ in 0..5 { c.run_pending_tasks(); }
    println!("retained after 500 fresh: {}", c.iter().count());
    let ctl: Cache<u32,u32> = Cache::new(1000);
    for k in 10000..10500 { ctl.insert(k, k); }
    for _ in 0..5 { ctl.run_pending_tasks(); }
    println!("control retained: {}", ctl.iter().count());

    let u: Cache<u32, u32> = Cache::builder().build();
    churn(&u, 4, 4);
    println!("unbounded: entry_count={} real={}", u.entry_count(), u.iter().count());
    let t: Cache<u32, u32> = Cache::builder().max_capacity(1000).time_to_live(std::time::Duration::from_millis(50)).build();
    churn(&t, 3, 4);
    std::thread::sleep(std::time::Duration::from_millis(200));
    for _ in 0..5 { t.run_pending_tasks(); }
    println!("ttl: entry_count={} real={}", t.entry_count(), t.iter().count());
    let l: Cache<u32, u32> = Cache::builder().max_capacity(1000).eviction_policy(EvictionPolicy::lru()).build();
    churn(&l, 4, 4);
    println!("lru cap1000: entry_count={} weighted={} real={}", l.entry_count(), l.weighted_size(), l.iter().count());
}
