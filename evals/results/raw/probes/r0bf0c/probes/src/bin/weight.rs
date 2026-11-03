// Concurrent updates of one key with differing weights: weighted_size vs real weight.
use moka::sync::Cache;
fn main() {
    for round in 0..6 {
        let cache: Cache<u32, u32> = Cache::builder().max_capacity(1_000_000).weigher(|_, v: &u32| *v).build();
        cache.insert(1, 1);
        cache.run_pending_tasks();
        let mut hs = vec![];
        for t in 0..8u32 {
            let c = cache.clone();
            hs.push(std::thread::spawn(move || {
                for i in 0..20000u32 { c.insert(1, 1 + (i * (t + 1)) % 1000); }
            }));
        }
        for h in hs { h.join().unwrap(); }
        for _ in 0..3 { cache.run_pending_tasks(); }
        let v = cache.get(&1).unwrap();
        println!("round {round}: weighted_size={} actual value/weight={} entry_count={}", cache.weighted_size(), v, cache.entry_count());
    }
}
