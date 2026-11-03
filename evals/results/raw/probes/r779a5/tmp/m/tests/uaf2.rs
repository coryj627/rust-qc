use moka::{sync::Cache, Expiry};
use std::time::{Duration, Instant};

struct Ex;
impl Expiry<u32, u32> for Ex {
    fn expire_after_create(&self, _: &u32, _: &u32, _: Instant) -> Option<Duration> {
        Some(if cfg!(miri) { Duration::from_secs(60) } else { Duration::from_millis(100) })
    }
    fn expire_after_read(&self, _: &u32, _: &u32, _: Instant, _: Option<Duration>, _: Instant) -> Option<Duration> {
        None
    }
}

// Single thread, no listener. The second replace_if call makes the internal
// unrecorded read hit the entry, which clears its expiry with no ReadOp.
#[test]
fn single_thread_dangling_timer_node() {
    let c: Cache<u32, u32> = Cache::builder().expire_after(Ex).build();
    c.insert(1, 1);
    c.run_pending_tasks();
    let mut calls = 0;
    let e = c.entry(1).or_insert_with_if(|| 2, |_| { calls += 1; calls == 1 });
    println!("value={} calls={calls}", e.value());
    std::thread::sleep(if cfg!(miri) { Duration::from_secs(65) } else { Duration::from_millis(1200) });
    c.run_pending_tasks();
    println!("after advance get={:?}", c.get(&1));
    c.invalidate(&1);
    c.run_pending_tasks();
    println!("end count={}", c.entry_count());
}
