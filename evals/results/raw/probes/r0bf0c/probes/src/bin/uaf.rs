// Expiry cleared by an unrecorded read, then the timer fires, then the entry is removed.
use moka::{sync::Cache, Expiry};
use std::time::{Duration, Instant};
struct E;
impl Expiry<u32, u32> for E {
    fn expire_after_create(&self, _: &u32, _: &u32, _: Instant) -> Option<Duration> { Some(Duration::from_secs(1)) }
    fn expire_after_read(&self, _: &u32, _: &u32, _: Instant, _: Option<Duration>, _: Instant) -> Option<Duration> { None }
}
fn main() {
    let cache: Cache<u32, u32> = Cache::builder().max_capacity(100).expire_after(E).build();
    cache.insert(1, 1);
    cache.run_pending_tasks();
    let mut n = 0;
    let e = cache.entry(1).or_insert_with_if(|| 99, |_| { n += 1; n == 1 });
    println!("calls={n} fresh={} v={}", e.is_fresh(), e.value());
    std::thread::sleep(Duration::from_millis(3300));
    cache.run_pending_tasks(); // timer node popped and freed
    println!("after advance get={:?}", cache.get(&1));
    cache.run_pending_tasks();
    cache.invalidate(&1);
    cache.run_pending_tasks(); // deschedule on dangling node
    println!("survived");
}
