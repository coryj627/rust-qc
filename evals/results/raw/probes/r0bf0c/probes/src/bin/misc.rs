use moka::{sync::Cache, Expiry, notification::RemovalCause};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
struct E;
impl Expiry<u32, u32> for E {
    fn expire_after_create(&self, _: &u32, _: &u32, _: Instant) -> Option<Duration> { Some(Duration::from_millis(100)) }
    fn expire_after_update(&self, _: &u32, _: &u32, _: Instant, _: Option<Duration>) -> Option<Duration> { Some(Duration::from_secs(100)) }
}
fn main() {
    // 1. remove() returning expired / invalidated values
    let c: Cache<u32, u32> = Cache::builder().time_to_live(Duration::from_millis(100)).build();
    c.insert(1, 1);
    std::thread::sleep(Duration::from_millis(200));
    println!("get expired={:?} contains={}", c.get(&1), c.contains_key(&1));
    println!("remove on expired returns {:?}", c.remove(&1));
    let c: Cache<u32, u32> = Cache::new(10);
    c.insert(1, 1);
    std::thread::sleep(Duration::from_millis(2));
    c.invalidate_all();
    println!("after invalidate_all get={:?} remove={:?}", c.get(&1), c.remove(&1));
    // 2. removal cause for per-entry expiry when replaced / invalidated
    let causes: Arc<Mutex<Vec<(u32, RemovalCause)>>> = Default::default();
    let cc = causes.clone();
    let c: Cache<u32, u32> = Cache::builder().expire_after(E).eviction_listener(move |k, _, cause| cc.lock().unwrap().push((*k, cause))).build();
    c.insert(1, 1);
    c.insert(2, 2);
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(200));
    println!("key1 get={:?}", c.get(&1));
    c.insert(1, 11); // replaces an expired entry
    c.invalidate(&2); // invalidates an expired entry
    c.run_pending_tasks();
    println!("causes (per-entry expired, replaced/invalidated): {:?}", causes.lock().unwrap());
    // same with TTL for comparison
    let causes: Arc<Mutex<Vec<(u32, RemovalCause)>>> = Default::default();
    let cc = causes.clone();
    let c: Cache<u32, u32> = Cache::builder().time_to_live(Duration::from_millis(100)).eviction_listener(move |k, _, cause| cc.lock().unwrap().push((*k, cause))).build();
    c.insert(1, 1);
    c.insert(2, 2);
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(200));
    c.insert(1, 11);
    c.invalidate(&2);
    c.run_pending_tasks();
    println!("causes (TTL control): {:?}", causes.lock().unwrap());
    // 3. predicate registered while another scan is finishing: predicate id semantics, and invalidate_entries_if with few entries
    let c: Cache<u32, u32> = Cache::builder().support_invalidation_closures().build();
    for k in 0..10 { c.insert(k, k); }
    c.run_pending_tasks();
    c.invalidate_entries_if(|k, _| *k < 5).unwrap();
    c.invalidate_entries_if(|k, _| *k >= 5).unwrap();
    for _ in 0..10 { c.run_pending_tasks(); }
    println!("two predicates small: count={}, visible={:?}", c.entry_count(), (0..10).filter(|k| c.get(k).is_some()).collect::<Vec<_>>());
}
