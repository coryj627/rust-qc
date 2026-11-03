use moka::{sync::Cache, Expiry};
use std::time::{Duration, Instant};
struct E;
impl Expiry<u32, u32> for E {
    fn expire_after_create(&self, _: &u32, _: &u32, _: Instant) -> Option<Duration> { Some(Duration::from_secs(5)) }
    fn expire_after_update(&self, _: &u32, _: &u32, _: Instant, _: Option<Duration>) -> Option<Duration> { Some(Duration::from_secs(10)) }
    fn expire_after_read(&self, _: &u32, _: &u32, _: Instant, _: Option<Duration>, _: Instant) -> Option<Duration> { Some(Duration::from_secs(20)) }
}
#[test]
fn basic_ops_with_expiry() {
    let c: Cache<u32, u32> = Cache::builder().max_capacity(4).expire_after(E).time_to_idle(Duration::from_secs(100)).time_to_live(Duration::from_secs(200)).support_invalidation_closures().build();
    for i in 0..6 { c.insert(i, i); }
    c.run_pending_tasks();
    for i in 0..6 { let _ = c.get(&i); let _ = c.get(&i); }
    c.run_pending_tasks();
    for i in 0..6 { c.insert(i, i + 1); }
    c.run_pending_tasks();
    c.invalidate(&0);
    c.run_pending_tasks();
    c.invalidate_entries_if(|k, _| *k == 1).unwrap();
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_secs(7));
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_secs(30));
    c.run_pending_tasks();
    for i in 10..14 { c.insert(i, i); }
    c.invalidate_all();
    std::thread::sleep(Duration::from_millis(5));
    c.run_pending_tasks();
    println!("count={}", c.entry_count());
}
