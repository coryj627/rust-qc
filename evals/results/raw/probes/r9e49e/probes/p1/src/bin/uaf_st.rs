// Single-threaded safe-code route to the dangling timer node pointer.
use moka::{sync::Cache, Expiry};
use std::time::{Duration, Instant};
struct E;
impl Expiry<u32, u32> for E {
    fn expire_after_create(&self, _: &u32, _: &u32, _: Instant) -> Option<Duration> { Some(Duration::from_millis(1500)) }
    fn expire_after_read(&self, _: &u32, _: &u32, _: Instant, _: Option<Duration>, _: Instant) -> Option<Duration> { None }
}
fn main() {
    let c: Cache<u32, u32> = Cache::builder().max_capacity(10).expire_after(E).build();
    c.insert(1, 1);
    c.run_pending_tasks();
    // First evaluation asks for replacement (so the recorded read is skipped), the
    // second keeps the value: that read goes through get_with_hash_without_recording.
    let mut n = 0;
    let v = c.get_with_if(1, || 2, |_| { n += 1; n == 1 });
    println!("get_with_if -> {v}");
    std::thread::sleep(Duration::from_millis(3500));
    c.run_pending_tasks();
    println!("timer advanced, invalidating");
    c.invalidate(&1);
    c.run_pending_tasks();
    println!("done");
}
