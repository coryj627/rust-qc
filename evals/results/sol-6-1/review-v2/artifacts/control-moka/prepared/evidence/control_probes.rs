// Negative-control regression probes for the unmodified moka 0.12.11 logic.
// Compiled as a cfg(test) sibling module in a scratch copy only.
use crate::{common::time::Clock, ops::compute::Op, sync::Cache};
use std::{future::Future, pin::Pin, sync::Arc, task::{Context, Poll, Wake, Waker}, time::Duration};

#[test]
fn decoy_1_zero_capacity_never_exposes_insert() {
    let cache = Cache::new(0);
    cache.insert("key", 7);
    assert_eq!(cache.get(&"key"), None);
    assert!(!cache.contains_key(&"key"));
    assert_eq!(cache.iter().count(), 0);
    cache.run_pending_tasks();
    assert_eq!(cache.entry_count(), 0);
    assert_eq!(cache.get(&"key"), None);
    println!("decoy-1: insert at capacity zero is invisible before and after maintenance; entry_count=0");
}

#[tokio::test]
async fn decoy_2_failed_initializer_is_not_cached() {
    let cache = crate::future::Cache::<&str, u32>::new(10);
    let first = cache.try_get_with("key", async { Err::<u32, &str>("temporary") }).await;
    assert_eq!(*first.unwrap_err(), "temporary");
    assert_eq!(cache.get(&"key").await, None);
    let called = std::sync::atomic::AtomicBool::new(false);
    let second = cache.try_get_with("key", async {
        called.store(true, std::sync::atomic::Ordering::SeqCst);
        Ok::<u32, &str>(42)
    }).await;
    assert_eq!(second.unwrap(), 42);
    assert!(called.load(std::sync::atomic::Ordering::SeqCst));
    assert_eq!(cache.get(&"key").await, Some(42));
    println!("decoy-2: Err(temporary) leaves a miss; next same-key/same-error-type initializer executes and caches 42");
}

#[test]
fn false_1_invalidate_all_hides_entries_before_maintenance() {
    let (clock, mock) = Clock::mock();
    let mut cache = Cache::builder().max_capacity(10).clock(clock).build();
    cache.reconfigure_for_testing(); // Disable automatic housekeeping only.
    cache.insert("old", 11);
    cache.run_pending_tasks();
    assert_eq!(cache.entry_count(), 1);
    mock.increment(Duration::from_secs(1));
    cache.invalidate_all();
    // No maintenance between invalidate_all and these observations.
    assert_eq!(cache.entry_count(), 1);
    assert_eq!(cache.get(&"old"), None);
    assert!(!cache.contains_key(&"old"));
    mock.increment(Duration::from_secs(1));
    cache.insert("new", 22);
    assert_eq!(cache.get(&"new"), Some(22));
    println!("false-1: stale physical entry remains counted, but get=None and contains_key=false before maintenance; later insertion readable");
}

#[test]
fn false_2_contains_key_does_not_refresh_idle_timer() {
    let (clock, mock) = Clock::mock();
    let mut cache = Cache::builder().max_capacity(10)
        .time_to_idle(Duration::from_secs(10)).clock(clock).build();
    cache.reconfigure_for_testing();
    cache.insert("key", 7);
    cache.run_pending_tasks();
    mock.increment(Duration::from_secs(9));
    assert!(cache.contains_key(&"key"));
    mock.increment(Duration::from_secs(1));
    assert!(!cache.contains_key(&"key"));
    assert_eq!(cache.get(&"key"), None);
    println!("false-2: contains_key=true at t=9s, false at t=10s; lookup did not extend the 10s idle deadline");
}

#[test]
fn false_3_iterator_filters_expired_physical_entries() {
    let (clock, mock) = Clock::mock();
    let mut cache = Cache::builder().max_capacity(10)
        .time_to_live(Duration::from_secs(10)).clock(clock).build();
    cache.reconfigure_for_testing();
    cache.insert("key", 7);
    cache.run_pending_tasks();
    assert_eq!(cache.entry_count(), 1);
    let mut iter = cache.iter();
    mock.increment(Duration::from_secs(10));
    assert_eq!(cache.entry_count(), 1);
    assert!(iter.next().is_none());
    assert_eq!(cache.iter().count(), 0);
    println!("false-3: at TTL boundary, physical entry_count=1 but existing and new iterators yield no entries without maintenance");
}

struct NoopWake;
impl Wake for NoopWake { fn wake(self: Arc<Self>) {} }

#[tokio::test]
async fn false_4_cancelled_initialization_does_not_strand_waiter() {
    let cache = crate::future::Cache::<&str, u32>::new(10);
    let waker = Waker::from(Arc::new(NoopWake));
    let mut cx = Context::from_waker(&waker);
    let entered = std::sync::atomic::AtomicBool::new(false);
    let mut abandoned = Box::pin(cache.get_with("key", async {
        entered.store(true, std::sync::atomic::Ordering::SeqCst);
        std::future::pending::<u32>().await
    }));
    assert!(matches!(Pin::as_mut(&mut abandoned).poll(&mut cx), Poll::Pending));
    assert!(entered.load(std::sync::atomic::Ordering::SeqCst));
    drop(abandoned); // Cancel after polling far enough to install the waiter.
    assert_eq!(cache.get(&"key").await, None);
    let value = tokio::time::timeout(Duration::from_secs(2), cache.get_with("key", async { 42 }))
        .await.expect("retry must complete after cancellation");
    assert_eq!(value, 42);
    assert_eq!(cache.get(&"key").await, Some(42));
    println!("false-4: polled pending initializer dropped; same-key retry completes with 42 and caches it");
}

#[test]
fn false_5_compute_error_preserves_existing_value() {
    let cache = Cache::new(10);
    cache.insert("key", 7);
    let failed = cache.entry("key").and_try_compute_with(|entry| {
        assert_eq!(entry.unwrap().into_value(), 7);
        Err::<Op<u32>, &str>("cannot update")
    });
    assert_eq!(failed.unwrap_err(), "cannot update");
    assert_eq!(cache.get(&"key"), Some(7));
    let retry = cache.entry("key").and_try_compute_with(|entry| {
        Ok::<Op<u32>, &str>(Op::Put(entry.unwrap().into_value() + 1))
    });
    assert!(retry.is_ok());
    assert_eq!(cache.get(&"key"), Some(8));
    println!("false-5: compute Err preserves 7; next compute reads 7 and successfully stores 8");
}
