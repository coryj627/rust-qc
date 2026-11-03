use moka::sync::Cache;
use moka::policy::EvictionPolicy;
use std::time::Duration;
fn ttl(n: u32) {
    let c: Cache<u32,u32> = Cache::builder().max_capacity(100).time_to_live(Duration::from_millis(200)).build();
    for i in 0..n { c.insert(1, i); if i % 100 == 0 { c.run_pending_tasks(); } }
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(400));
    for _ in 0..5 { c.run_pending_tasks(); }
    println!("ttl updates={n}: entry_count={} get={:?} iter={}", c.entry_count(), c.get(&1), c.iter().count());
}
fn lru(n: u32) {
    let c: Cache<u32,u32> = Cache::builder().max_capacity(10).eviction_policy(EvictionPolicy::lru()).build();
    for i in 0..n { c.insert(1, i); if i % 100 == 0 { c.run_pending_tasks(); } }
    c.run_pending_tasks();
    for k in 2..300 { c.insert(k, k); c.run_pending_tasks(); }
    for _ in 0..5 { c.run_pending_tasks(); }
    println!("lru updates={n}: hot key present={} entry_count={}", c.contains_key(&1), c.entry_count());
}
fn main() { ttl(60000); ttl(70000); lru(60000); lru(70000); }
