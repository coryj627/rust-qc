// Re-insert over an expired-but-not-yet-evicted entry, with Expiry that overrides only create.
use moka::{sync::Cache, Expiry};
use std::time::{Duration, Instant};
struct E;
impl Expiry<u32, u32> for E {
    fn expire_after_create(&self, _: &u32, _: &u32, _: Instant) -> Option<Duration> { Some(Duration::from_millis(100)) }
}
fn main() {
    let cache: Cache<u32, u32> = Cache::builder().expire_after(E).build();
    cache.insert(1, 1);
    cache.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(200));
    println!("expired get={:?}", cache.get(&1));
    cache.insert(1, 2);
    println!("immediately after re-insert get={:?} (expect Some(2))", cache.get(&1));
    let v = cache.get_with(5, || 5);
    println!("get_with v={v}");
    std::thread::sleep(Duration::from_millis(200));
    let v = cache.get_with(5, || 6);
    println!("get_with after expiry v={v} next get={:?} contains={}", cache.get(&5), cache.contains_key(&5));
}
