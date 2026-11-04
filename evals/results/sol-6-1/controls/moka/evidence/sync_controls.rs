use super::Cache;
use crate::common::time::Clock;
use std::{cell::Cell, time::Duration};

#[test]
fn decoy_1_zero_capacity_skips_storage() {
    let mut cache = Cache::new(0);
    cache.reconfigure_for_testing();
    cache.insert("key", 41);
    assert_eq!(cache.get(&"key"), None);
    assert!(!cache.contains_key(&"key"));
    assert_eq!(cache.iter().count(), 0);
    cache.run_pending_tasks();
    assert_eq!(cache.entry_count(), 0);
    assert_eq!(cache.weighted_size(), 0);
    assert_eq!(cache.get(&"key"), None);
    println!("decoy-1: insert into capacity 0 is invisible before and after maintenance; count=0, weight=0");
}

#[test]
fn decoy_2_failed_initializer_can_be_retried() {
    let cache = Cache::new(100);
    let calls = Cell::new(0);
    let first = cache.try_get_with("key", || {
        calls.set(calls.get() + 1);
        Err::<u32, &str>("temporary failure")
    });
    assert_eq!(*first.unwrap_err(), "temporary failure");
    assert_eq!(cache.get(&"key"), None);
    assert!(cache.is_waiter_map_empty());
    let second = cache.try_get_with("key", || {
        calls.set(calls.get() + 1);
        Ok::<u32, &str>(42)
    });
    assert_eq!(second.unwrap(), 42);
    assert_eq!(calls.get(), 2);
    assert_eq!(cache.get(&"key"), Some(42));
    assert!(cache.is_waiter_map_empty());
    println!("decoy-2: Err leaves no cached value or waiter; same key/error type retries successfully; calls=2");
}

#[test]
fn false_2_contains_key_does_not_reset_idle_clock() {
    let (clock, mock) = Clock::mock();
    let mut cache = Cache::builder()
        .max_capacity(100)
        .time_to_idle(Duration::from_secs(10))
        .clock(clock)
        .build();
    cache.reconfigure_for_testing();
    cache.insert("contains", 1);
    cache.insert("get", 2);
    cache.run_pending_tasks();
    mock.increment(Duration::from_secs(6));
    assert!(cache.contains_key(&"contains"));
    assert_eq!(cache.get(&"get"), Some(2));
    mock.increment(Duration::from_secs(3));
    assert!(cache.contains_key(&"contains"));
    mock.increment(Duration::from_secs(1));
    assert!(!cache.contains_key(&"contains"));
    assert_eq!(cache.get(&"contains"), None);
    assert_eq!(cache.get(&"get"), Some(2));
    println!("false-2: contains at t=6,9 does not extend TTI=10; expires at t=10; ordinary get at t=6 does extend TTI");
}

#[test]
fn false_4_admitted_overwrite_replaces_weight() {
    let mut cache = Cache::builder()
        .max_capacity(100)
        .weigher(|_key: &&str, value: &u32| *value)
        .build();
    cache.reconfigure_for_testing();
    cache.insert("key", 60);
    cache.run_pending_tasks();
    assert_eq!((cache.entry_count(), cache.weighted_size()), (1, 60));
    cache.insert("key", 30);
    cache.run_pending_tasks();
    assert_eq!((cache.entry_count(), cache.weighted_size()), (1, 30));
    cache.insert("other", 60);
    cache.run_pending_tasks();
    assert_eq!((cache.entry_count(), cache.weighted_size()), (2, 90));
    assert_eq!(cache.get(&"key"), Some(30));
    assert_eq!(cache.get(&"other"), Some(60));
    println!("false-4: admitted overwrite weight 60 -> 30 gives weight=30; adding weight=60 retains both entries with total=90");
}
