// Expiry: updating an entry whose per-entry expiry has passed
use moka::{sync::Cache, Expiry, notification::RemovalCause};
use std::{sync::{Arc, Mutex}, time::{Duration, Instant}};
struct OnlyCreate;
impl Expiry<u32, u32> for OnlyCreate {
    fn expire_after_create(&self, _: &u32, _: &u32, _: Instant) -> Option<Duration> { Some(Duration::from_millis(500)) }
}
struct UpdateNone;
impl Expiry<u32, u32> for UpdateNone {
    fn expire_after_create(&self, _: &u32, _: &u32, _: Instant) -> Option<Duration> { Some(Duration::from_millis(500)) }
    fn expire_after_update(&self, _: &u32, _: &u32, _: Instant, _: Option<Duration>) -> Option<Duration> { None }
}
fn main() {
    let c: Cache<u32, u32> = Cache::builder().expire_after(OnlyCreate).build();
    c.insert(1, 1);
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(700));
    c.insert(1, 2);
    println!("default update, old expired: get right after insert = {:?}", c.get(&1));

    let c: Cache<u32, u32> = Cache::builder().expire_after(UpdateNone).build();
    c.insert(1, 1);
    c.run_pending_tasks();
    c.insert(1, 2);
    println!("update->None before old expiry: get = {:?}", c.get(&1));
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(700));
    println!("  after 700ms (no expiry requested) = {:?}", c.get(&1));

    let c: Cache<u32, u32> = Cache::builder().expire_after(UpdateNone).build();
    c.insert(1, 1);
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(700));
    c.insert(1, 2);
    println!("update->None after old expiry (user asks never expire): get = {:?}", c.get(&1));

    // removal cause
    let log = Arc::new(Mutex::new(Vec::new()));
    let l2 = log.clone();
    let c: Cache<u32, u32> = Cache::builder().expire_after(UpdateNone)
        .eviction_listener(move |k, v, cause| l2.lock().unwrap().push((*k, v, cause))).build();
    c.insert(1, 1);
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(700));
    c.insert(1, 2);
    c.insert(2, 2);
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(0));
    println!("cause when replacing per-entry expired value: {:?}", *log.lock().unwrap());
    log.lock().unwrap().clear();
    let c2: Cache<u32, u32> = { let l2 = log.clone(); Cache::builder().expire_after(OnlyCreate)
        .eviction_listener(move |k, v, cause| l2.lock().unwrap().push((*k, v, cause))).build() };
    c2.insert(5, 5);
    c2.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(700));
    c2.invalidate(&5);
    c2.run_pending_tasks();
    println!("cause when invalidating per-entry expired value: {:?}", *log.lock().unwrap());
    let _ = RemovalCause::Expired;
}
