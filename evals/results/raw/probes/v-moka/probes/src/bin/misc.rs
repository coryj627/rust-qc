use moka::{sync::{Cache, SegmentedCache}, Expiry, notification::RemovalCause};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use moka::ops::compute::{Op, CompResult};
struct E;
impl Expiry<u32, u32> for E {
    fn expire_after_create(&self, _: &u32, _: &u32, _: Instant) -> Option<Duration> { Some(Duration::from_millis(50)) }
    fn expire_after_update(&self, _: &u32, _: &u32, _: Instant, _: Option<Duration>) -> Option<Duration> { Some(Duration::from_secs(50)) }
}
fn main() {
    // by_ref key
    let c: Cache<String,u32> = Cache::new(10);
    c.insert("a".into(), 1);
    let e = c.entry_by_ref("a").or_try_insert_with(|| Ok::<u32,()>(2)).unwrap();
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| e.key().clone()));
    println!("entry_by_ref or_try_insert_with hit key panic={}", r.is_err());
    let e = c.entry("a".to_string()).or_try_insert_with(|| Ok::<u32,()>(2)).unwrap();
    println!("control owned key={}", e.key());
    // remove
    let c: Cache<u32,u32> = Cache::builder().max_capacity(10).time_to_live(Duration::from_millis(50)).build();
    c.insert(1,1); std::thread::sleep(Duration::from_millis(100));
    println!("after TTL get={:?} remove={:?}", c.get(&1), c.remove(&1));
    let c: Cache<u32,u32> = Cache::new(10);
    c.insert(1,1); std::thread::sleep(Duration::from_millis(1)); c.invalidate_all();
    println!("after invalidate_all get={:?} remove={:?}", c.get(&1), c.remove(&1));
    c.insert(2,2); std::thread::sleep(Duration::from_millis(1)); c.invalidate_all();
    let r = c.entry(2).and_compute_with(|e| { println!("compute saw {:?}", e.map(|e| e.into_value())); Op::Remove });
    println!("compute result {}", match r { CompResult::Removed(e)=>format!("Removed({})", e.into_value()), CompResult::StillNone(_)=>"StillNone".into(), _=>"other".into() });
    // cause
    let causes = Arc::new(Mutex::new(vec![]));
    let cc = causes.clone();
    let c: Cache<u32,u32> = Cache::builder().max_capacity(10).expire_after(E).eviction_listener(move |k, _, cause: RemovalCause| cc.lock().unwrap().push((*k,cause))).build();
    c.insert(1,1); c.insert(2,2); std::thread::sleep(Duration::from_millis(100));
    c.insert(1,10); c.invalidate(&2);
    println!("per-entry expiry causes {:?}", causes.lock().unwrap());
    let causes = Arc::new(Mutex::new(vec![]));
    let cc = causes.clone();
    let c: Cache<u32,u32> = Cache::builder().max_capacity(10).time_to_live(Duration::from_millis(50)).eviction_listener(move |k, _, cause: RemovalCause| cc.lock().unwrap().push((*k,cause))).build();
    c.insert(1,1); c.insert(2,2); std::thread::sleep(Duration::from_millis(100));
    c.insert(1,10); c.invalidate(&2);
    println!("ttl control causes {:?}", causes.lock().unwrap());
    // is_old_value_replaced
    let c: Cache<u32,u32> = Cache::new(10);
    c.insert(1,1);
    let e = c.entry(1).or_insert_with_if(|| 2, |_| true);
    println!("or_insert_with_if value={} fresh={} old_replaced={}", e.value(), e.is_fresh(), e.is_old_value_replaced());
    // segmented
    let s: SegmentedCache<u32,u32> = SegmentedCache::new(10, 4);
    for k in 0..2000 { s.insert(k,k); if k % 50 == 0 {s.run_pending_tasks();} }
    s.run_pending_tasks();
    println!("segmented cap 10/4: entry_count={} iter={}", s.entry_count(), s.iter().count());
    let s: SegmentedCache<u32,u32> = SegmentedCache::new(8, 16);
    for k in 0..2000 { s.insert(k,k); if k % 50 == 0 {s.run_pending_tasks();} }
    s.run_pending_tasks();
    println!("segmented cap 8/16: entry_count={} iter={}", s.entry_count(), s.iter().count());
}
