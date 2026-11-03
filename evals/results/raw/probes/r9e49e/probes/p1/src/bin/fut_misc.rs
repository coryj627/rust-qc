use moka::future::Cache;
use std::{time::Duration, sync::{Arc, Mutex}};
use futures_util::FutureExt as _;
#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    // A. get_with waiting on try_get_with with E = ()
    let c: Cache<u32, u32> = Cache::new(100);
    let c2 = c.clone();
    let t = tokio::spawn(async move { c2.try_get_with(1u32, async { tokio::time::sleep(Duration::from_millis(300)).await; Err::<u32, ()>(()) }).await });
    tokio::time::sleep(Duration::from_millis(50)).await;
    let r = std::panic::AssertUnwindSafe(c.get_with(1, async { 7 })).catch_unwind().await;
    println!("A future get_with vs try_get_with<()>: {:?}", r.map_err(|_| "PANIC"));
    println!("  {:?}", t.await.unwrap());

    // B. predicates
    let c: Cache<u32, u32> = Cache::builder().support_invalidation_closures().build();
    for i in 0..2000 { c.insert(i, i).await; }
    c.run_pending_tasks().await;
    c.invalidate_entries_if(|_, _| true).unwrap();
    for _ in 0..20 { c.run_pending_tasks().await; }
    let mut left = vec![];
    for i in 0..2000 { if c.contains_key(&i) { left.push(i); } }
    println!("B future predicate survivors: {:?}", left);

    // C. eviction listener with cancellation: listener future cancelled mid-way
    let log = Arc::new(Mutex::new(Vec::new()));
    let l2 = log.clone();
    let c: Cache<u32, u32> = Cache::builder().max_capacity(100)
        .async_eviction_listener(move |k, v, cause| { let l = l2.clone(); futures_util::FutureExt::boxed(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            l.lock().unwrap().push((*k, v, cause));
        }) }).build();
    c.insert(1, 1).await;
    c.run_pending_tasks().await;
    let _ = tokio::time::timeout(Duration::from_millis(5), c.insert(1, 2)).await;
    println!("C after cancelled insert get={:?} log={:?}", c.get(&1).await, *log.lock().unwrap());
    tokio::time::sleep(Duration::from_millis(100)).await;
    let _ = tokio::time::timeout(Duration::from_millis(5), c.invalidate(&1)).await;
    println!("  after cancelled invalidate get={:?}", c.get(&1).await);
    c.run_pending_tasks().await;
    tokio::time::sleep(Duration::from_millis(150)).await;
    c.run_pending_tasks().await;
    println!("  log={:?} count={}", *log.lock().unwrap(), c.entry_count());
}
