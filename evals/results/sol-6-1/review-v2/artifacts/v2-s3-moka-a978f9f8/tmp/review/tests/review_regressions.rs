use moka::{future, sync};

#[test]
fn sync_generation_wrap_cleanup() {
    let mut counts = Vec::new();
    for updates in [65_534_u32, 65_535, 65_536] {
        let cache = sync::Cache::new(10);
        cache.insert(0, 0);
        cache.run_pending_tasks();
        for value in 1..=updates {
            cache.insert(0, value);
            cache.run_pending_tasks();
        }
        cache.invalidate_all();
        for _ in 0..4 {
            cache.run_pending_tasks();
        }
        assert_eq!(cache.get(&0), None);
        counts.push(cache.entry_count());
    }
    assert_eq!(
        counts,
        [0, 0, 0],
        "cleanup after 65_534, 65_535, and 65_536 updates"
    );
}

#[tokio::test]
async fn future_generation_wrap_cleanup() {
    let mut counts = Vec::new();
    for updates in [65_534_u32, 65_535, 65_536] {
        let cache = future::Cache::new(10);
        cache.insert(0, 0).await;
        cache.run_pending_tasks().await;
        for value in 1..=updates {
            cache.insert(0, value).await;
            cache.run_pending_tasks().await;
        }
        cache.invalidate_all();
        for _ in 0..4 {
            cache.run_pending_tasks().await;
        }
        assert_eq!(cache.get(&0).await, None);
        counts.push(cache.entry_count());
    }
    assert_eq!(
        counts,
        [0, 0, 0],
        "cleanup after 65_534, 65_535, and 65_536 updates"
    );
}

struct CreateExpiry;
impl moka::Expiry<u32, u32> for CreateExpiry {
    fn expire_after_create(
        &self,
        _: &u32,
        _: &u32,
        _: std::time::Instant,
    ) -> Option<std::time::Duration> {
        Some(std::time::Duration::from_millis(20))
    }
    fn expire_after_update(
        &self,
        _: &u32,
        _: &u32,
        _: std::time::Instant,
        _: Option<std::time::Duration>,
    ) -> Option<std::time::Duration> {
        None
    }
}

#[test]
fn sync_reinsert_expired_custom_expiry() {
    let cache = sync::Cache::builder().expire_after(CreateExpiry).build();
    cache.insert(0, 1);
    assert_eq!(cache.get(&0), Some(1));
    std::thread::sleep(std::time::Duration::from_millis(40));
    assert_eq!(cache.get(&0), None);
    cache.insert(0, 2);
    assert_eq!(
        cache.get(&0),
        Some(2),
        "newly inserted value must be available"
    );
}

#[tokio::test]
async fn future_reinsert_expired_custom_expiry() {
    let cache = future::Cache::builder().expire_after(CreateExpiry).build();
    cache.insert(0, 1).await;
    assert_eq!(cache.get(&0).await, Some(1));
    tokio::time::sleep(std::time::Duration::from_millis(40)).await;
    assert_eq!(cache.get(&0).await, None);
    cache.insert(0, 2).await;
    assert_eq!(
        cache.get(&0).await,
        Some(2),
        "newly inserted value must be available"
    );
}

#[test]
fn sync_compute_remove_invalidated_entry() {
    let cache = sync::Cache::new(10);
    cache.insert(0, 1);
    cache.invalidate_all();
    let result = cache.entry(0).and_compute_with(|entry| {
        assert!(entry.is_none());
        moka::ops::compute::Op::Remove
    });
    assert!(
        matches!(result, moka::ops::compute::CompResult::StillNone(_)),
        "result={result:?}"
    );
}

#[tokio::test]
async fn future_compute_remove_invalidated_entry() {
    let cache = future::Cache::new(10);
    cache.insert(0, 1).await;
    cache.invalidate_all();
    let result = cache
        .entry(0)
        .and_compute_with(|entry| async move {
            assert!(entry.is_none());
            moka::ops::compute::Op::Remove
        })
        .await;
    assert!(
        matches!(result, moka::ops::compute::CompResult::StillNone(_)),
        "result={result:?}"
    );
}

#[test]
fn sync_remove_invalidated_entry() {
    let cache = sync::Cache::new(10);
    cache.insert(0, 1);
    cache.invalidate_all();
    assert_eq!(cache.remove(&0), None);
}

#[tokio::test]
async fn future_remove_invalidated_entry() {
    let cache = future::Cache::new(10);
    cache.insert(0, 1).await;
    cache.invalidate_all();
    assert_eq!(cache.remove(&0).await, None);
}

#[test]
fn sync_borrowed_fallible_entry_hit_has_key() {
    let cache: sync::Cache<String, u32> = sync::Cache::new(10);
    let first = cache
        .entry_by_ref("key")
        .or_try_insert_with(|| Ok::<_, ()>(1))
        .unwrap();
    assert_eq!(first.key(), "key");
    let hit = cache
        .entry_by_ref("key")
        .or_try_insert_with(|| Ok::<_, ()>(2))
        .unwrap();
    assert_eq!(hit.value(), &1);
    assert_eq!(hit.key(), "key");
}

#[test]
fn segmented_borrowed_fallible_entry_hit_has_key() {
    let cache: sync::SegmentedCache<String, u32> = sync::SegmentedCache::new(10, 2);
    cache.insert("key".to_owned(), 1);
    let hit = cache
        .entry_by_ref("key")
        .or_try_insert_with(|| Ok::<_, ()>(2))
        .unwrap();
    assert_eq!(hit.value(), &1);
    assert_eq!(hit.key(), "key");
}

#[tokio::test]
async fn future_borrowed_fallible_entry_hit_has_key() {
    let cache: future::Cache<String, u32> = future::Cache::new(10);
    cache.insert("key".to_owned(), 1).await;
    let hit = cache
        .entry_by_ref("key")
        .or_try_insert_with(async { Ok::<_, ()>(2) })
        .await
        .unwrap();
    assert_eq!(hit.value(), &1);
    assert_eq!(hit.key(), "key");
}

#[tokio::test]
async fn future_compute_updates_remain_serial_while_listener_waits() {
    use std::{sync::Arc, task::Poll};
    let release = Arc::new(tokio::sync::Notify::new());
    let listener_release = release.clone();
    let cache = future::Cache::builder()
        .async_eviction_listener(move |_, value, _| {
            let release = listener_release.clone();
            Box::pin(async move {
                if value == 0 {
                    release.notified().await;
                }
            })
        })
        .build();
    cache.insert(0, 0).await;
    let increment =
        |entry: Option<moka::Entry<u32, u32>>| async move { entry.unwrap().into_value() + 1 };
    let first = cache.entry(0).and_upsert_with(increment);
    let second = cache.entry(0).and_upsert_with(increment);
    let third = cache.entry(0).and_upsert_with(increment);
    futures_util::pin_mut!(first, second, third);
    assert!(matches!(futures_util::poll!(&mut first), Poll::Pending));
    assert!(matches!(futures_util::poll!(&mut second), Poll::Pending));
    assert!(matches!(futures_util::poll!(&mut third), Poll::Pending));
    release.notify_one();
    let _ = tokio::join!(first, second, third);
    assert_eq!(cache.get(&0).await, Some(3));
}

#[test]
fn sync_weight_tracks_latest_value_when_write_logs_arrive_out_of_order() {
    use std::sync::{Arc, Barrier};
    struct PauseSecond {
        entered: Arc<Barrier>,
        release: Arc<Barrier>,
    }
    impl moka::Expiry<u32, u32> for PauseSecond {
        fn expire_after_update(
            &self,
            _: &u32,
            value: &u32,
            _: std::time::Instant,
            current: Option<std::time::Duration>,
        ) -> Option<std::time::Duration> {
            if *value == 2 {
                self.entered.wait();
                self.release.wait();
            }
            current
        }
    }
    let entered = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let cache = sync::Cache::builder()
        .weigher(|_: &u32, value: &u32| *value)
        .expire_after(PauseSecond {
            entered: entered.clone(),
            release: release.clone(),
        })
        .build();
    cache.insert(0, 1);
    cache.run_pending_tasks();
    let worker_cache = cache.clone();
    let worker = std::thread::spawn(move || worker_cache.insert(0, 2));
    entered.wait();
    cache.insert(0, 3);
    cache.run_pending_tasks();
    release.wait();
    worker.join().unwrap();
    cache.run_pending_tasks();
    assert_eq!(cache.get(&0), Some(3));
    assert_eq!(cache.entry_count(), 1);
    let weight_before_removal = cache.weighted_size();
    cache.invalidate(&0);
    cache.run_pending_tasks();
    assert_eq!((weight_before_removal, cache.weighted_size()), (3, 0));
}

#[tokio::test]
async fn future_weight_tracks_latest_value_when_write_logs_arrive_out_of_order() {
    use std::sync::{Arc, Barrier};
    struct PauseSecond {
        entered: Arc<Barrier>,
        release: Arc<Barrier>,
    }
    impl moka::Expiry<u32, u32> for PauseSecond {
        fn expire_after_update(
            &self,
            _: &u32,
            value: &u32,
            _: std::time::Instant,
            current: Option<std::time::Duration>,
        ) -> Option<std::time::Duration> {
            if *value == 2 {
                self.entered.wait();
                self.release.wait();
            }
            current
        }
    }
    let entered = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let cache = future::Cache::builder()
        .weigher(|_: &u32, value: &u32| *value)
        .expire_after(PauseSecond {
            entered: entered.clone(),
            release: release.clone(),
        })
        .build();
    cache.insert(0, 1).await;
    cache.run_pending_tasks().await;
    let worker_cache = cache.clone();
    let worker = std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(worker_cache.insert(0, 2))
    });
    entered.wait();
    cache.insert(0, 3).await;
    cache.run_pending_tasks().await;
    release.wait();
    worker.join().unwrap();
    cache.run_pending_tasks().await;
    assert_eq!(cache.get(&0).await, Some(3));
    assert_eq!(cache.entry_count(), 1);
    let weight_before_removal = cache.weighted_size();
    cache.invalidate(&0).await;
    cache.run_pending_tasks().await;
    assert_eq!((weight_before_removal, cache.weighted_size()), (3, 0));
}
