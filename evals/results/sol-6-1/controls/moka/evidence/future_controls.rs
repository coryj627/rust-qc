use super::Cache;
use crate::common::time::Clock;
use std::{future::pending, time::Duration};

#[tokio::test]
async fn false_1_cancelled_initializer_releases_existing_follower() {
    let cache = Cache::new(100);
    let mut first = Box::pin(cache.get_with("key", pending::<u32>()));
    assert!(futures_util::poll!(first.as_mut()).is_pending());
    assert!(!cache.is_waiter_map_empty());
    let mut follower = Box::pin(cache.get_with("key", async { 77 }));
    assert!(futures_util::poll!(follower.as_mut()).is_pending());
    drop(first);
    assert!(cache.is_waiter_map_empty());
    let result = tokio::time::timeout(Duration::from_secs(2), follower).await;
    assert_eq!(result.expect("follower stuck after cancellation"), 77);
    assert_eq!(cache.get(&"key").await, Some(77));
    assert!(cache.is_waiter_map_empty());
    println!("false-1: polled initializer and follower both pending; drop initializer clears waiter and existing follower completes with 77");
}

#[tokio::test]
async fn false_3_invalidation_preserves_later_writes() {
    let (clock, mock) = Clock::mock();
    let mut cache = Cache::builder().max_capacity(100).clock(clock).build();
    cache.reconfigure_for_testing().await;
    cache.insert("old", 1).await;
    cache.insert("replaced", 2).await;
    cache.run_pending_tasks().await;
    mock.increment(Duration::from_secs(1));
    cache.invalidate_all();
    assert_eq!(cache.get(&"old").await, None);
    mock.increment(Duration::from_secs(1));
    cache.insert("new", 3).await;
    cache.insert("replaced", 4).await;
    assert_eq!(cache.get(&"new").await, Some(3));
    assert_eq!(cache.get(&"replaced").await, Some(4));
    cache.run_pending_tasks().await;
    cache.run_pending_tasks().await;
    assert_eq!(cache.get(&"old").await, None);
    assert_eq!(cache.get(&"new").await, Some(3));
    assert_eq!(cache.get(&"replaced").await, Some(4));
    assert_eq!(cache.entry_count(), 2);
    println!("false-3: invalidate_all at t=1 removes pre-cutoff value; new key and replacement written at t=2 survive delayed maintenance; count=2");
}

#[tokio::test]
async fn false_5_iterator_filters_ttl_without_maintenance() {
    let (clock, mock) = Clock::mock();
    let mut cache = Cache::builder()
        .max_capacity(100)
        .time_to_live(Duration::from_secs(10))
        .clock(clock)
        .build();
    cache.reconfigure_for_testing().await;
    cache.insert("expired", 1).await;
    mock.increment(Duration::from_secs(5));
    cache.insert("live", 2).await;
    cache.run_pending_tasks().await;
    assert_eq!(cache.entry_count(), 2);
    let iter = cache.iter();
    mock.increment(Duration::from_secs(5));
    // No maintenance after advancing to exactly the older entry's TTL.
    let items: Vec<_> = iter.map(|(k, v)| (*k, v)).collect();
    assert_eq!(items, vec![("live", 2)]);
    assert_eq!(cache.entry_count(), 2);
    assert_eq!(cache.get(&"expired").await, None);
    assert_eq!(cache.get(&"live").await, Some(2));
    println!("false-5: at exact TTL t=10 iterator yields only live entry; entry_count remains 2, so expired entry was filtered without maintenance");
}
