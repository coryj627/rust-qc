use std::time::{Duration, Instant};
struct Exp;
impl moka::Expiry<u32,u32> for Exp {
    fn expire_after_create(&self, _: &u32, _: &u32, _: Instant) -> Option<Duration> { Some(Duration::from_secs(2)) }
    fn expire_after_read(&self, _: &u32, _: &u32, _: Instant, _: Option<Duration>, _: Instant) -> Option<Duration> { None }
}
fn main() {
    let c = moka::sync::Cache::builder().max_capacity(100).expire_after(Exp).build();
    c.insert(1u32,1u32);
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(1200));
    println!("get {:?}", c.get(&1));
    c.run_pending_tasks();
    println!("count {}", c.entry_count());
    c.invalidate(&1);
    c.run_pending_tasks();
}
