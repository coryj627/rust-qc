// Single-threaded: expire_after_read clears the expiration during an unrecorded read.
use moka::{sync::Cache, Expiry};
use std::time::{Duration, Instant};
struct E;
impl Expiry<u32, u32> for E {
    fn expire_after_create(&self, _: &u32, _: &u32, _: Instant) -> Option<Duration> {
        Some(Duration::from_millis(500))
    }
    fn expire_after_read(&self, _: &u32, _: &u32, _: Instant, _: Option<Duration>, _: Instant) -> Option<Duration> {
        None
    }
}
fn main() {
    let c: Cache<u32, u32> = Cache::builder().max_capacity(100).expire_after(E).build();
    c.insert(1, 1);
    c.run_pending_tasks();
    let mut n = 0;
    // 1st replace_if (recorded) -> true means ignore; 2nd (unrecorded read in waiter path) -> false means keep.
    let v = c.entry(1).or_insert_with_if(|| 2, |_| { n += 1; n == 1 });
    println!("entry {:?} fresh {} replace_if calls {n}", v.value(), v.is_fresh());
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(3500));
    c.run_pending_tasks();
    println!("after tick");
    let junk: Vec<Vec<u64>> = (0..2000).map(|i| vec![0xdead_beef_dead_beef + i; 6]).collect();
    println!("junk {}", junk.len());
    c.invalidate(&1);
    c.run_pending_tasks();
    println!("done, entry_count={}", c.entry_count());
}
