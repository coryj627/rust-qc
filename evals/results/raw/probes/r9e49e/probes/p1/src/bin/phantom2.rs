// What phantoms do to a cache afterwards.
use moka::sync::Cache;
fn churn(c: &Cache<u32, u32>, ops: u32) {
    let mut ts = vec![];
    for t in 0..4u32 {
        let c = c.clone();
        ts.push(std::thread::spawn(move || {
            let mut x = 12345u32.wrapping_add(t * 977);
            for _ in 0..ops {
                x = x.wrapping_mul(1664525).wrapping_add(1013904223);
                let k = (x >> 16) % 16;
                if x & 1 == 0 { c.insert(k, x); } else { c.invalidate(&k); }
            }
        }));
    }
    for t in ts { t.join().unwrap(); }
    for k in 0..16 { c.invalidate(&k); }
    for _ in 0..5 { c.run_pending_tasks(); }
}
fn main() {
    // bounded cache
    let c: Cache<u32, u32> = Cache::builder().max_capacity(1000).build();
    churn(&c, 400_000);
    println!("bounded after churn: entry_count={} weighted={} actual={}", c.entry_count(), c.weighted_size(), c.iter().count());
    for k in 100..1100 { c.insert(k, k); }
    c.run_pending_tasks();
    let hits = (100..1100).filter(|k| c.get(k).is_some()).count();
    println!("  insert 1000 fresh keys into capacity-1000 cache: {} retained, entry_count={} weighted={} actual={}", hits, c.entry_count(), c.weighted_size(), c.iter().count());
    // control
    let c: Cache<u32, u32> = Cache::builder().max_capacity(1000).build();
    for k in 100..1100 { c.insert(k, k); }
    c.run_pending_tasks();
    let hits = (100..1100).filter(|k| c.get(k).is_some()).count();
    println!("  control without churn: {} retained", hits);
    // unbounded grows
    let c: Cache<u32, u32> = Cache::builder().build();
    for round in 0..4 {
        churn(&c, 200_000);
        println!("unbounded round {round}: entry_count={} weighted={} actual={}", c.entry_count(), c.weighted_size(), c.iter().count());
    }
    // no invalidate: two inserters only
    let c: Cache<u32, u32> = Cache::builder().max_capacity(1000).time_to_live(std::time::Duration::from_millis(100)).build();
    churn(&c, 200_000);
    std::thread::sleep(std::time::Duration::from_millis(300));
    for _ in 0..5 { c.run_pending_tasks(); }
    println!("ttl 100ms, after expiry: entry_count={} weighted={} actual={}", c.entry_count(), c.weighted_size(), c.iter().count());
}
