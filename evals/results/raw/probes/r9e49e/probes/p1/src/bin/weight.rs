use moka::sync::Cache;
use std::sync::{Arc, Barrier};
fn main() {
    // A. concurrent first inserts of the same keys (no invalidate): op reorder counts weight twice.
    for round in 0..3 {
        let c: Cache<u32, u32> = Cache::builder().build();
        let b = Arc::new(Barrier::new(4));
        let mut ts = vec![];
        for t in 0..4u32 {
            let (c, b) = (c.clone(), b.clone());
            ts.push(std::thread::spawn(move || { b.wait(); for k in 0..300_000u32 { c.insert(k, t); } }));
        }
        for t in ts { t.join().unwrap(); }
        for _ in 0..5 { c.run_pending_tasks(); }
        println!("A round {round}: 300000 keys, weight 1 each: entry_count={} weighted_size={} actual={}", c.entry_count(), c.weighted_size(), c.iter().count());
    }
    // B. concurrent updates with changing weight on one key: retry reads the mutated weight.
    for round in 0..3 {
        let c: Cache<u32, u32> = Cache::builder().weigher(|_, v: &u32| *v).build();
        c.insert(1, 1);
        c.insert(2, 1);
        c.run_pending_tasks();
        let mut ts = vec![];
        for t in 0..4u32 {
            let c = c.clone();
            ts.push(std::thread::spawn(move || {
                let mut x = 7u32 + t;
                for i in 0..300_000u32 {
                    x = x.wrapping_mul(1664525).wrapping_add(1013904223);
                    c.insert(1 + (i & 1), 1 + (x >> 24));
                }
            }));
        }
        for t in ts { t.join().unwrap(); }
        for _ in 0..5 { c.run_pending_tasks(); }
        c.insert(1, 5);
        c.insert(2, 5);
        for _ in 0..5 { c.run_pending_tasks(); }
        println!("B round {round}: two keys with final weight 5 each: weighted_size={} (expected 10)", c.weighted_size());
    }
    // C. weight drift then capacity: same as B on bounded cache
    let c: Cache<u32, u32> = Cache::builder().time_to_live(std::time::Duration::from_millis(300)).build();
    c.insert(1, 1);
    std::thread::sleep(std::time::Duration::from_millis(500));
    println!("C ttl-expired remove first: remove={:?}", c.remove(&1));
}
