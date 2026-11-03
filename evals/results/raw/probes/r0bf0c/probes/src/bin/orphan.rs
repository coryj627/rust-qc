// insert/invalidate race on few keys: entries in map vs entries counted.
use moka::sync::Cache;
fn main() {
    let mut bad = 0;
    for round in 0..20 {
        let cache: Cache<u32, u32> = Cache::builder().max_capacity(1000).build();
        let mut hs = vec![];
        for t in 0..8u32 {
            let c = cache.clone();
            hs.push(std::thread::spawn(move || {
                for i in 0..30000u32 {
                    let k = i % 4;
                    if (i + t) % 3 == 0 { c.invalidate(&k); } else { c.insert(k, i); }
                }
            }));
        }
        for h in hs { h.join().unwrap(); }
        for _ in 0..3 { cache.run_pending_tasks(); }
        let n = cache.iter().count() as u64;
        if n != cache.entry_count() { bad += 1; println!("round {round}: iter={} entry_count={}", n, cache.entry_count()); }
    }
    println!("mismatching rounds: {bad}");
}
