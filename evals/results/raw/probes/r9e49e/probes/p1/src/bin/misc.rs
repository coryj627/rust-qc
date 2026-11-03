use moka::{sync::Cache, ops::compute::Op, Expiry};
use std::{time::{Duration, Instant}, panic::{catch_unwind, AssertUnwindSafe}, sync::{Arc, atomic::{AtomicBool, Ordering}}};
struct MaxExp;
impl Expiry<u32, u32> for MaxExp {
    fn expire_after_create(&self, _: &u32, _: &u32, _: Instant) -> Option<Duration> { Some(Duration::MAX) }
}
struct UpdZero;
impl Expiry<u32, u32> for UpdZero {
    fn expire_after_create(&self, _: &u32, _: &u32, _: Instant) -> Option<Duration> { Some(Duration::from_secs(100)) }
}
fn main() {
    std::panic::set_hook(Box::new(|i| println!("  panic: {}", i)));
    // remove returns expired / invalidated values
    let c: Cache<u32, u32> = Cache::builder().time_to_live(Duration::from_millis(200)).build();
    c.insert(1, 1);
    std::thread::sleep(Duration::from_millis(400));
    println!("ttl expired: get={:?} contains={} remove={:?}", c.get(&1), c.contains_key(&2), c.remove(&1));
    let c: Cache<u32, u32> = Cache::new(10);
    c.insert(1, 1);
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(5));
    c.invalidate_all();
    println!("invalidate_all: get={:?} remove={:?}", c.get(&1), c.remove(&1));
    let c: Cache<u32, u32> = Cache::builder().support_invalidation_closures().build();
    c.insert(1, 1);
    c.run_pending_tasks();
    c.invalidate_entries_if(|_, _| true).unwrap();
    println!("predicate: get={:?} remove={:?}", c.get(&1), c.remove(&1));
    // compute Op::Remove on expired
    let c: Cache<u32, u32> = Cache::builder().time_to_live(Duration::from_millis(200)).build();
    c.insert(1, 1);
    std::thread::sleep(Duration::from_millis(400));
    let r = c.entry(1).and_compute_with(|e| { println!("  compute sees {:?}", e.map(|e| e.into_value())); Op::Remove });
    println!("compute Remove on expired: {:?}", r);

    // get_key/iter
    // is_old_value_replaced
    let c: Cache<u32, u32> = Cache::new(10);
    c.insert(1, 1);
    let e = c.entry(1).or_insert_with_if(|| 2, |_| true);
    println!("or_insert_with_if replaced: value={} fresh={} old_replaced={}", e.value(), e.is_fresh(), e.is_old_value_replaced());

    // Expiry Duration::MAX
    let c: Cache<u32, u32> = Cache::builder().expire_after(MaxExp).build();
    c.insert(1, 1);
    let r = catch_unwind(AssertUnwindSafe(|| { let g = c.get(&1); c.run_pending_tasks(); c.insert(1, 2); c.run_pending_tasks(); (g, c.get(&1)) }));
    println!("Duration::MAX expiry: {:?}", r.map_err(|_| "PANIC"));

    // one run_pending_tasks after predicate
    let c: Cache<u32, u32> = Cache::builder().support_invalidation_closures().build();
    for i in 0..2000 { c.insert(i, i); }
    c.run_pending_tasks();
    c.invalidate_entries_if(|_, _| true).unwrap();
    c.run_pending_tasks();
    println!("one run_pending_tasks after predicate on 2000 entries: entry_count={}", c.entry_count());

    // same for invalidate_all
    let c: Cache<u32, u32> = Cache::new(5000);
    for i in 0..2000 { c.insert(i, i); }
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(2));
    c.invalidate_all();
    c.run_pending_tasks();
    println!("one run_pending_tasks after invalidate_all on 2000 entries: entry_count={}", c.entry_count());
    let _ = UpdZero;

    // weigher panic inside compute: next compute spins forever?
    let c: Cache<u32, u32> = Cache::builder().max_capacity(100).weigher(|_, v: &u32| { if *v == 13 { panic!("bad weight") } 1 }).build();
    let r = catch_unwind(AssertUnwindSafe(|| c.entry(1).and_upsert_with(|_| 13).into_value()));
    println!("upsert with panicking weigher: {:?}", r.map_err(|_| "PANIC"));
    let done = Arc::new(AtomicBool::new(false));
    let d2 = done.clone();
    let c2 = c.clone();
    std::thread::spawn(move || { let v = c2.entry(1).and_upsert_with(|_| 14).into_value(); d2.store(true, Ordering::SeqCst); println!("second upsert returned {v}"); });
    std::thread::sleep(Duration::from_secs(3));
    println!("second upsert finished within 3 s: {}", done.load(Ordering::SeqCst));
    std::process::exit(0);
}
