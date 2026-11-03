use moka::notification::RemovalCause;
use moka::sync::{Cache, SegmentedCache};
use moka::Expiry;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

struct E;
impl Expiry<u32, Arc<u32>> for E {
    fn expire_after_create(&self, _: &u32, v: &Arc<u32>, _: Instant) -> Option<Duration> { if **v % 3 == 0 { Some(Duration::from_millis(1)) } else { None } }
    fn expire_after_read(&self, _: &u32, v: &Arc<u32>, _: Instant, d: Option<Duration>, _: Instant) -> Option<Duration> { if **v % 5 == 0 { Some(Duration::from_millis(2)) } else if **v % 7 == 0 { None } else { d } }
    fn expire_after_update(&self, _: &u32, v: &Arc<u32>, _: Instant, d: Option<Duration>) -> Option<Duration> { if **v % 2 == 0 { Some(Duration::from_millis(1)) } else { d } }
}

fn main() {
    let n = Arc::new(AtomicU64::new(0));
    let n2 = n.clone();
    let c: Cache<u32, Arc<u32>> = Cache::builder()
        .max_capacity(8)
        .expire_after(E)
        .time_to_idle(Duration::from_millis(50))
        .time_to_live(Duration::from_millis(100))
        .support_invalidation_closures()
        .eviction_listener(move |_k, _v, _c: RemovalCause| { n2.fetch_add(1, Ordering::Relaxed); })
        .build();
    for i in 0..60u32 {
        let k = i % 11;
        c.insert(k, Arc::new(i));
        let _ = c.get(&(k / 2));
        if i % 4 == 0 { c.invalidate(&(k / 3)); }
        if i % 9 == 0 { c.run_pending_tasks(); }
        if i == 30 { c.invalidate_entries_if(|k, _| k % 2 == 0).unwrap(); }
        if i == 40 { c.invalidate_all(); }
        if i % 13 == 0 { std::thread::sleep(Duration::from_millis(2)); }
    }
    c.run_pending_tasks();
    let c2 = c.clone();
    let t = std::thread::spawn(move || { for i in 0..20u32 { c2.insert(i % 3, Arc::new(i)); let _ = c2.get(&1); } });
    for i in 0..20u32 { c.insert(i % 3, Arc::new(i)); c.invalidate(&(i % 3)); }
    t.join().unwrap();
    let _ = c.iter().count();
    c.run_pending_tasks();
    drop(c);
    let s: SegmentedCache<u32, String> = SegmentedCache::builder(2).max_capacity(4).build();
    for i in 0..300u32 { s.insert(i % 200, format!("{i}")); }
    s.run_pending_tasks();
    println!("ok listener calls {}", n.load(Ordering::Relaxed));
}
