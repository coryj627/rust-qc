// Ghost entries from insert/invalidate racing on the same key, and what they do to the cache afterwards.
use moka::sync::Cache;
fn churn(cache: &Cache<u32, u32>, per_thread: u32) {
    let mut hs = vec![];
    for t in 0..8u32 {
        let c = cache.clone();
        hs.push(std::thread::spawn(move || {
            for i in 0..per_thread {
                let k = i % 4;
                if (i + t) % 3 == 0 { c.invalidate(&k); } else { c.insert(k, i); }
            }
        }));
    }
    for h in hs { h.join().unwrap(); }
    for _ in 0..3 { cache.run_pending_tasks(); }
}
fn usable(cache: &Cache<u32, u32>) -> usize {
    // Remove what is left, then try to use the cache normally
    for k in 0..4 { cache.invalidate(&k); }
    cache.run_pending_tasks();
    println!("  after invalidating all 4 keys: map entries={} entry_count={} weighted_size={}", cache.iter().count(), cache.entry_count(), cache.weighted_size());
    for round in 0..5 {
        for k in 1000..1500u32 { cache.insert(k, k); let _ = cache.get(&k); }
        cache.run_pending_tasks();
        let _ = round;
    }
    (1000..1500u32).filter(|k| cache.contains_key(k)).count()
}
fn main() {
    let c1: Cache<u32, u32> = Cache::builder().max_capacity(1000).eviction_policy(moka::policy::EvictionPolicy::tiny_lfu()).build();
    println!("control (no churn): fresh cache cap 1000 retains {} of 500 new keys", usable(&c1));
    let c2: Cache<u32, u32> = Cache::builder().max_capacity(1000).build();
    churn(&c2, 30000);
    println!("after churn: map={} entry_count={}", c2.iter().count(), c2.entry_count());
    println!("churned cache cap 1000 retains {} of 500 new keys", usable(&c2));
    let c3: Cache<u32, u32> = Cache::builder().build();
    for n in 0..3 { churn(&c3, 30000); println!("unbounded after churn {n}: map={} entry_count={}", c3.iter().count(), c3.entry_count()); }
    let c4: Cache<u32, u32> = Cache::builder().time_to_live(std::time::Duration::from_millis(50)).build();
    churn(&c4, 30000);
    std::thread::sleep(std::time::Duration::from_millis(200));
    for _ in 0..30 { c4.run_pending_tasks(); }
    println!("TTL 50ms after 200ms idle: map={} entry_count={}", c4.iter().count(), c4.entry_count());
}
