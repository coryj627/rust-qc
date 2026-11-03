// policy_gen wrap: a key updated > 65535 times.
use moka::sync::Cache;
use std::time::Duration;
fn main() {
    // A: TTL cleanup
    let cache: Cache<u32, u32> = Cache::builder().time_to_live(Duration::from_millis(200)).build();
    for i in 0..70_000u32 { cache.insert(1, i); }
    cache.run_pending_tasks();
    println!("A count before ttl = {}", cache.entry_count());
    std::thread::sleep(Duration::from_millis(400));
    for _ in 0..5 { cache.run_pending_tasks(); }
    println!("A count after ttl (expect 0) = {}, get={:?}", cache.entry_count(), cache.get(&1));
    // control
    let c2: Cache<u32, u32> = Cache::builder().time_to_live(Duration::from_millis(200)).build();
    for i in 0..1_000u32 { c2.insert(1, i); }
    c2.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(400));
    for _ in 0..5 { c2.run_pending_tasks(); }
    println!("A-control count after ttl = {}", c2.entry_count());
    // B: size eviction in LRU
    let c3: Cache<u32, u32> = Cache::builder().max_capacity(10).eviction_policy(moka::policy::EvictionPolicy::lru()).build();
    for i in 0..70_000u32 { c3.insert(1, i); }
    c3.run_pending_tasks();
    for k in 100..300u32 { c3.insert(k, k); c3.run_pending_tasks(); }
    println!("B hot key still present after 200 newer inserts under LRU cap 10 = {:?}, count={}", c3.get(&1).is_some(), c3.entry_count());
    let c4: Cache<u32, u32> = Cache::builder().max_capacity(10).eviction_policy(moka::policy::EvictionPolicy::lru()).build();
    for i in 0..1_000u32 { c4.insert(1, i); }
    c4.run_pending_tasks();
    for k in 100..300u32 { c4.insert(k, k); c4.run_pending_tasks(); }
    println!("B-control = {:?}", c4.get(&1).is_some());
}
