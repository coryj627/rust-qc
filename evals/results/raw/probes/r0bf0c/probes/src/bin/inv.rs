// invalidate_entries_if over more entries than one eviction batch (384).
use moka::sync::Cache;
fn main() {
    let cache: Cache<u32, u32> = Cache::builder().max_capacity(10_000).support_invalidation_closures().build();
    let mut i = 0;
    for k in 0..2000u32 { cache.insert(k, k); i += 1; if i % 50 == 0 { cache.run_pending_tasks(); } }
    cache.run_pending_tasks();
    println!("before: count={}", cache.entry_count());
    cache.invalidate_entries_if(|_, _| true).unwrap();
    for _ in 0..50 { cache.run_pending_tasks(); }
    println!("after: count={}", cache.entry_count());
    let alive: Vec<u32> = (0..2000).filter(|k| cache.get(k).is_some()).collect();
    println!("visible again after invalidate_entries_if(true): {alive:?}");
}
