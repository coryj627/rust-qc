// Only insert/get/run_pending_tasks + a slow eviction listener. Second thread's plain `get`
// clears the expiration between apply_reads and timer advance.
use moka::{sync::Cache, Expiry};
use std::time::{Duration, Instant};
const TICK: u64 = 1 << 30; // ns
struct E(Instant);
impl Expiry<u32, u32> for E {
    fn expire_after_create(&self, k: &u32, _: &u32, now: Instant) -> Option<Duration> {
        if *k == 2 {
            // Mid tick 1 after wheel origin (roughly cache build time).
            Some((self.0 + Duration::from_nanos(TICK + TICK * 3 / 4)).saturating_duration_since(now))
        } else { None }
    }
    fn expire_after_read(&self, _: &u32, _: &u32, _: Instant, _: Option<Duration>, _: Instant) -> Option<Duration> { None }
}
fn main() {
    let t0 = Instant::now();
    let c: Cache<u32, u32> = Cache::builder().max_capacity(100).weigher(|_, v: &u32| *v)
        .expire_after(E(t0))
        .eviction_listener(|k, _, cause| { println!("listener k={k} {cause:?}"); std::thread::sleep(Duration::from_millis(250)); })
        .build();
    c.insert(2, 1);
    c.run_pending_tasks(); // admit + schedule in tick 0.
    std::thread::sleep((t0 + Duration::from_nanos(TICK + TICK / 20)).saturating_duration_since(Instant::now()));
    let c2 = c.clone();
    let reader = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(80));
        println!("reader get(2)={:?}", c2.get(&2));
    });
    // Oversized candidate: rejected inside apply_writes, runs the listener before timer advance.
    c.insert(3, 1000);
    c.run_pending_tasks();
    reader.join().unwrap();
    println!("rpt1 done");
    let junk: Vec<Vec<u64>> = (0..2000).map(|i| vec![0xdead_beef_dead_beef + i; 6]).collect();
    println!("junk {}", junk.len());
    c.run_pending_tasks();
    c.invalidate(&2);
    c.run_pending_tasks();
    println!("done, entry_count={}", c.entry_count());
}
