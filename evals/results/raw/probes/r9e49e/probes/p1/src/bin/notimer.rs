// Expiry set by a read whose ReadOp is not recorded: no timer ever exists.
use moka::{sync::Cache, Expiry};
use std::time::{Duration, Instant};
struct E;
impl Expiry<u32, u32> for E {
    fn expire_after_read(&self, _: &u32, _: &u32, _: Instant, _: Option<Duration>, _: Instant) -> Option<Duration> { Some(Duration::from_millis(100)) }
}
fn main() {
    let c: Cache<u32, u32> = Cache::builder().expire_after(E).build();
    c.insert(1, 1);
    c.run_pending_tasks();
    let mut n = 0;
    let e = c.entry(1).or_insert_with_if(|| 2, |_| { n += 1; n == 1 });
    println!("or_insert_with_if -> {}", e.value());
    std::thread::sleep(Duration::from_secs(4));
    for _ in 0..5 { c.run_pending_tasks(); std::thread::sleep(Duration::from_millis(1200)); }
    println!("after expiry: get={:?} entry_count={} iter={}", c.get(&1), c.entry_count(), c.iter().count());
}
