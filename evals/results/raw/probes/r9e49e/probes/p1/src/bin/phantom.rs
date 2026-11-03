// entry_count / weighted_size must match actual content after quiescing.
use moka::sync::Cache;
use std::sync::Arc;
fn main() {
    for round in 0..6 {
        let c: Cache<u32, u32> = Cache::builder().max_capacity(1000).build();
        let mut ts = vec![];
        for t in 0..4u32 {
            let c = c.clone();
            ts.push(std::thread::spawn(move || {
                let mut x = 12345u32.wrapping_add(t * 977);
                for _ in 0..400_000 {
                    x = x.wrapping_mul(1664525).wrapping_add(1013904223);
                    let k = (x >> 16) % 16;
                    if x & 1 == 0 { c.insert(k, x); } else { c.invalidate(&k); }
                }
            }));
        }
        for t in ts { t.join().unwrap(); }
        for _ in 0..5 { c.run_pending_tasks(); }
        let actual = c.iter().count();
        println!("round {round}: entry_count={} weighted={} actual={}", c.entry_count(), c.weighted_size(), actual);
        // Now remove all and see what remains
        for k in 0..16 { c.invalidate(&k); }
        for _ in 0..5 { c.run_pending_tasks(); }
        println!("   after invalidating all: entry_count={} weighted={} actual={}", c.entry_count(), c.weighted_size(), c.iter().count());
    }
    let _ = Arc::new(0);
}
