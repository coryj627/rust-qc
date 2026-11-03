// The same scenarios through future::Cache.
use moka::{future::Cache, Expiry};
use std::time::{Duration, Instant};
struct E;
impl Expiry<u32, u32> for E {
    fn expire_after_create(&self, _: &u32, _: &u32, _: Instant) -> Option<Duration> { Some(Duration::from_millis(100)) }
}
#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let cache: Cache<u32, u32> = Cache::builder().max_capacity(10_000).support_invalidation_closures().build();
    for k in 0..2000u32 { cache.insert(k, k).await; if k % 50 == 0 { cache.run_pending_tasks().await; } }
    cache.run_pending_tasks().await;
    cache.invalidate_entries_if(|_, _| true).unwrap();
    for _ in 0..50 { cache.run_pending_tasks().await; }
    let mut alive = vec![];
    for k in 0..2000u32 { if cache.get(&k).await.is_some() { alive.push(k); } }
    println!("future invalidate_entries_if: resurrected {alive:?}");

    let cache: Cache<u32, u32> = Cache::builder().expire_after(E).build();
    cache.insert(1, 1).await;
    cache.run_pending_tasks().await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    cache.insert(1, 2).await;
    println!("future re-insert over expired: get={:?}", cache.get(&1).await);

    let cache: Cache<u32, u32> = Cache::builder().time_to_live(Duration::from_millis(200)).build();
    for i in 0..70_000u32 { cache.insert(1, i).await; }
    cache.run_pending_tasks().await;
    tokio::time::sleep(Duration::from_millis(400)).await;
    for _ in 0..5 { cache.run_pending_tasks().await; }
    println!("future gen wrap: entry_count after ttl={}", cache.entry_count());

    let cache: Cache<u32, u32> = Cache::builder().max_capacity(1000).build();
    let mut hs = vec![];
    for t in 0..8u32 {
        let c = cache.clone();
        hs.push(tokio::spawn(async move {
            for i in 0..30000u32 {
                let k = i % 4;
                if (i + t) % 3 == 0 { c.invalidate(&k).await; } else { c.insert(k, i).await; }
            }
        }));
    }
    for h in hs { h.await.unwrap(); }
    for _ in 0..3 { cache.run_pending_tasks().await; }
    println!("future ghost: map={} entry_count={}", cache.iter().count(), cache.entry_count());
}
