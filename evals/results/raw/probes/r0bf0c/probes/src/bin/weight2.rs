// As weight.rs, but a large unrelated entry keeps the total far from 0, so saturation cannot be the cause.
use moka::sync::Cache;
fn main() {
    for round in 0..5 {
        let cache: Cache<u32, u32> = Cache::builder().max_capacity(u64::MAX / 2).weigher(|_, v: &u32| *v).build();
        cache.insert(2, 1_000_000_000);
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
        let v = cache.get(&1).unwrap() as u64;
        let expect = 1_000_000_000u64 + v;
        println!("round {round}: weighted_size={} expected={} drift={}", cache.weighted_size(), expect, cache.weighted_size() as i64 - expect as i64);
    }
}
