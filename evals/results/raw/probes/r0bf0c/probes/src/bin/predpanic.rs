// A predicate that panics once during housekeeping.
use moka::future::Cache;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use futures_util::FutureExt;
use std::panic::AssertUnwindSafe;
#[tokio::main(flavor = "current_thread")]
async fn main() {
    std::panic::set_hook(Box::new(|i| eprintln!("PANIC: {}", i.to_string().replace('\n', " "))));
    let cache: Cache<u32, u32> = Cache::builder().max_capacity(100).support_invalidation_closures().build();
    for k in 0..5 { cache.insert(k, k).await; }
    cache.run_pending_tasks().await;
    let flag = Arc::new(AtomicBool::new(true));
    let f = flag.clone();
    // Panics only during the housekeeper scan's first call
    cache.invalidate_entries_if(move |_, _| { if f.swap(false, Ordering::SeqCst) { panic!("predicate once") } false }).unwrap();
    let r = AssertUnwindSafe(cache.run_pending_tasks()).catch_unwind().await;
    println!("first run_pending_tasks panicked: {}", r.is_err());
    for i in 0..3 {
        let r = AssertUnwindSafe(cache.run_pending_tasks()).catch_unwind().await;
        println!("run_pending_tasks {i} after predicate stopped panicking: panicked={}", r.is_err());
    }
    let r = AssertUnwindSafe(async {
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        cache.insert(100, 100).await;
        cache.get(&100).await
    }).catch_unwind().await;
    println!("insert/get later: {:?}", r.map_err(|_| "panicked"));

    // sync
    let c: moka::sync::Cache<u32, u32> = moka::sync::Cache::builder().max_capacity(100).support_invalidation_closures().build();
    for k in 0..5 { c.insert(k, k); }
    let flag = Arc::new(AtomicBool::new(true));
    c.run_pending_tasks();
    println!("sync before: {}", c.entry_count());
    for k in 5..10 { c.insert(k, k); }
    let f = flag.clone();
    c.invalidate_entries_if(move |_, _| { if f.swap(false, Ordering::SeqCst) { panic!("predicate once") } false }).unwrap();
    let r = std::panic::catch_unwind(AssertUnwindSafe(|| c.run_pending_tasks()));
    println!("sync first run panicked: {}", r.is_err());
    c.run_pending_tasks();
    c.run_pending_tasks();
    println!("sync after: entry_count={} map={}", c.entry_count(), c.iter().count());
}
