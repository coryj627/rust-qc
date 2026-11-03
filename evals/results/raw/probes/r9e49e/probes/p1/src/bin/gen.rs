// entry generation wraparound after 65536 updates of one key
use moka::sync::Cache;
use std::time::Duration;
fn main() {
    // A. invalidate_entries_if does not see a key after >65535 updates
    let c: Cache<u32, u32> = Cache::builder().support_invalidation_closures().build();
    c.insert(999, 0);
    for i in 0..65_540u32 {
        c.insert(1, i);
        if i % 256 == 0 { c.run_pending_tasks(); }
    }
    c.run_pending_tasks();
    c.invalidate_entries_if(|_, _| true).unwrap();
    for _ in 0..5 { c.run_pending_tasks(); }
    println!("A after predicate: key1={:?} key999={:?} count={}", c.get(&1), c.get(&999), c.entry_count());

    // B. TTL: entry never evicted
    let c: Cache<u32, u32> = Cache::builder().time_to_live(Duration::from_secs(1)).build();
    for i in 0..65_540u32 {
        c.insert(1, i);
        if i % 256 == 0 { c.run_pending_tasks(); }
    }
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_secs(2));
    for _ in 0..5 { c.run_pending_tasks(); }
    println!("B after TTL: get={:?} entry_count={} (expected 0)", c.get(&1), c.entry_count());

    // C. size eviction: hot key can't be evicted
    let c: Cache<u32, u32> = Cache::builder().max_capacity(1).eviction_policy(moka::policy::EvictionPolicy::lru()).build();
    for i in 0..65_540u32 {
        c.insert(1, i);
        if i % 256 == 0 { c.run_pending_tasks(); }
    }
    c.run_pending_tasks();
    for k in 10..200 { c.insert(k, k); c.run_pending_tasks(); }
    println!("C capacity 1: count={} ws={} key1 present={}", c.entry_count(), c.weighted_size(), c.contains_key(&1));

    // control
    let c: Cache<u32, u32> = Cache::builder().time_to_live(Duration::from_secs(1)).build();
    for i in 0..60_000u32 {
        c.insert(1, i);
        if i % 256 == 0 { c.run_pending_tasks(); }
    }
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_secs(2));
    for _ in 0..5 { c.run_pending_tasks(); }
    println!("control 60k updates after TTL: entry_count={} (expected 0)", c.entry_count());
}
