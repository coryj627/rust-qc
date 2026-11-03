//! moka-5: expire_after_update returning None does not clear the expiration of an
//! entry that has already expired, so the value just written stays unreadable.
//! Exit 0 = bug shown (get returns None after an update that asked for no expiry).
use moka::{sync::Cache, Expiry};
use std::time::{Duration, Instant};

struct E;
impl Expiry<u32, &'static str> for E {
    fn expire_after_create(&self, _: &u32, _: &&'static str, _: Instant) -> Option<Duration> {
        Some(Duration::from_millis(50))
    }
    fn expire_after_update(
        &self,
        _: &u32,
        _: &&'static str,
        _: Instant,
        remaining: Option<Duration>,
    ) -> Option<Duration> {
        println!("expire_after_update called: duration_until_expiry={remaining:?}, returning None");
        None // Documented meaning: the entry does not expire.
    }
}

fn main() {
    let cache: Cache<u32, &'static str> = Cache::builder().max_capacity(100).expire_after(E).build();
    cache.insert(1, "first");
    cache.run_pending_tasks();
    assert_eq!(cache.get(&1), Some("first"));

    std::thread::sleep(Duration::from_millis(120)); // expired, not yet evicted
    cache.insert(1, "second"); // expire_after_update -> None
    let immediately = cache.get(&1);
    cache.run_pending_tasks();
    let after_housekeeping = cache.get(&1);
    println!("get right after update      = {immediately:?} (expected Some(\"second\"))");
    println!("get after run_pending_tasks = {after_housekeeping:?} (expected Some(\"second\"))");
    println!("entry_count                 = {}", cache.entry_count());
    if immediately.is_none() && after_housekeeping.is_none() {
        println!("BUG: the entry updated with no expiration is unreadable");
        std::process::exit(0);
    }
    println!("no bug: the updated entry is readable");
    std::process::exit(1);
}
