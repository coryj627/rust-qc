#[cfg(test)]
mod tests {
    use moka::Expiry;
    use std::time::{Duration, Instant};

    struct ClearOnUpdate;
    impl Expiry<u32, u32> for ClearOnUpdate {
        fn expire_after_create(&self, _: &u32, value: &u32, _: Instant) -> Option<Duration> {
            if *value == 0 {
                Some(Duration::ZERO)
            } else {
                None
            }
        }
        fn expire_after_update(
            &self,
            _: &u32,
            _: &u32,
            _: Instant,
            _: Option<Duration>,
        ) -> Option<Duration> {
            None
        }
    }

    #[test]
    fn sync_clear_expired_deadline_on_update() {
        let cache = moka::sync::Cache::builder()
            .expire_after(ClearOnUpdate)
            .build();
        cache.insert(1, 0);
        assert_eq!(cache.get(&1), None);
        cache.insert(1, 1);
        assert_eq!(cache.get(&1), Some(1));
    }

    #[tokio::test]
    async fn future_clear_expired_deadline_on_update() {
        let cache = moka::future::Cache::builder()
            .expire_after(ClearOnUpdate)
            .build();
        cache.insert(1, 0).await;
        assert_eq!(cache.get(&1).await, None);
        cache.insert(1, 1).await;
        assert_eq!(cache.get(&1).await, Some(1));
    }

    #[test]
    fn sync_remove_after_invalidate_all() {
        let cache = moka::sync::Cache::new(10);
        cache.insert(1, 42);
        cache.invalidate_all();
        assert_eq!(cache.get(&1), None);
        assert_eq!(cache.remove(&1), None);
    }

    #[tokio::test]
    async fn future_remove_after_invalidate_all() {
        let cache = moka::future::Cache::new(10);
        cache.insert(1, 42).await;
        cache.invalidate_all();
        assert_eq!(cache.get(&1).await, None);
        assert_eq!(cache.remove(&1).await, None);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_future_upserts_preserve_all_increments() {
        let cache = moka::future::Cache::new(100);
        cache.insert(1, 0u64).await;
        let mut tasks = Vec::new();
        for _ in 0..4 {
            let cache = cache.clone();
            tasks.push(tokio::spawn(async move {
                for _ in 0..1000 {
                    cache
                        .entry(1)
                        .and_upsert_with(|entry| async move { entry.unwrap().into_value() + 1 })
                        .await;
                }
            }));
        }
        for task in tasks {
            task.await.unwrap();
        }
        assert_eq!(cache.get(&1).await, Some(4000));
    }

    #[test]
    fn sync_infallible_loader_survives_unit_error_loader() {
        let cache = moka::sync::Cache::new(10);
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let c1 = cache.clone();
        let fallible = std::thread::spawn(move || {
            c1.try_get_with(1, || {
                started_tx.send(()).unwrap();
                release_rx.recv().unwrap();
                Err::<u32, ()>(())
            })
        });
        started_rx.recv().unwrap();
        let c2 = cache.clone();
        let infallible = std::thread::spawn(move || c2.get_with(1, || 7));
        std::thread::sleep(Duration::from_millis(30));
        release_tx.send(()).unwrap();
        assert!(fallible.join().unwrap().is_err());
        assert_eq!(infallible.join().unwrap(), 7);
    }

    #[tokio::test]
    async fn future_infallible_loader_survives_unit_error_loader() {
        let cache = moka::future::Cache::new(10);
        let (fallible, infallible) = tokio::join!(
            cache.try_get_with(1, async {
                tokio::time::sleep(Duration::from_millis(30)).await;
                Err::<u32, ()>(())
            }),
            cache.get_with(1, async { 7 })
        );
        assert!(fallible.is_err());
        assert_eq!(infallible, 7);
    }

    #[test]
    fn sync_generation_wrap_does_not_prevent_cleanup() {
        let mut counts = Vec::new();
        for writes in [65_535, 65_536, 65_537] {
            let cache = moka::sync::Cache::new(10);
            for value in 0..writes {
                cache.insert(1, value);
            }
            cache.run_pending_tasks();
            assert_eq!(cache.entry_count(), 1);
            cache.invalidate_all();
            for _ in 0..3 {
                cache.run_pending_tasks();
            }
            assert_eq!(cache.get(&1), None);
            counts.push((writes, cache.entry_count()));
        }
        assert_eq!(counts, vec![(65_535, 0), (65_536, 0), (65_537, 0)]);
    }

    #[tokio::test]
    async fn future_generation_wrap_does_not_prevent_cleanup() {
        let mut counts = Vec::new();
        for writes in [65_535, 65_536, 65_537] {
            let cache = moka::future::Cache::new(10);
            for value in 0..writes {
                cache.insert(1, value).await;
            }
            cache.run_pending_tasks().await;
            assert_eq!(cache.entry_count(), 1);
            cache.invalidate_all();
            for _ in 0..3 {
                cache.run_pending_tasks().await;
            }
            assert_eq!(cache.get(&1).await, None);
            counts.push((writes, cache.entry_count()));
        }
        assert_eq!(counts, vec![(65_535, 0), (65_536, 0), (65_537, 0)]);
    }

    #[tokio::test]
    async fn future_upserts_hold_lock_until_write_finishes() {
        use std::{
            future::{poll_fn, Future},
            sync::{Arc, Mutex},
            task::Poll,
        };
        let release_listener = Arc::new(tokio::sync::Notify::new());
        let release = release_listener.clone();
        let cache = moka::future::Cache::builder()
            .async_eviction_listener(move |_, value, _| {
                let release = release.clone();
                Box::pin(async move {
                    if value == 0 {
                        release.notified().await;
                    }
                })
            })
            .build();
        cache.insert(1, 0u32).await;
        let blocker = cache.insert(1, 10);
        tokio::pin!(blocker);
        poll_fn(|cx| {
            assert!(blocker.as_mut().poll(cx).is_pending());
            Poll::Ready(())
        })
        .await;
        let seen = Arc::new(Mutex::new(Vec::new()));
        let seen_a = seen.clone();
        let first = cache.entry(1).and_upsert_with(move |entry| async move {
            let old = entry.unwrap().into_value();
            seen_a.lock().unwrap().push(old);
            old + 1
        });
        tokio::pin!(first);
        poll_fn(|cx| {
            assert!(first.as_mut().poll(cx).is_pending());
            Poll::Ready(())
        })
        .await;
        let seen_b = seen.clone();
        let second = cache.entry(1).and_upsert_with(move |entry| async move {
            let old = entry.unwrap().into_value();
            seen_b.lock().unwrap().push(old);
            old + 1
        });
        tokio::pin!(second);
        poll_fn(|cx| {
            assert!(second.as_mut().poll(cx).is_pending());
            Poll::Ready(())
        })
        .await;
        release_listener.notify_one();
        blocker.await;
        first.await;
        second.await;
        let final_value = cache.get(&1).await;
        println!("inputs={:?}, final={final_value:?}", *seen.lock().unwrap());
        assert_eq!(*seen.lock().unwrap(), vec![10, 11]);
        assert_eq!(cache.get(&1).await, Some(12));
    }

    #[test]
    fn sync_compute_remove_reports_logically_absent_entry() {
        use moka::ops::compute::{CompResult, Op};
        let cache = moka::sync::Cache::new(10);
        cache.insert(1, 42);
        cache.invalidate_all();
        let result = cache.entry(1).and_compute_with(|entry| {
            assert!(entry.is_none());
            Op::Remove
        });
        assert!(matches!(result, CompResult::StillNone(_)), "{result:?}");
    }

    #[tokio::test]
    async fn future_compute_remove_reports_logically_absent_entry() {
        use moka::ops::compute::{CompResult, Op};
        let cache = moka::future::Cache::new(10);
        cache.insert(1, 42).await;
        cache.invalidate_all();
        let result = cache
            .entry(1)
            .and_compute_with(|entry| async move {
                assert!(entry.is_none());
                Op::Remove
            })
            .await;
        assert!(matches!(result, CompResult::StillNone(_)), "{result:?}");
    }

    #[test]
    fn control_sync_concurrent_upserts_preserve_all_increments() {
        let cache = moka::sync::Cache::new(100);
        cache.insert(1, 0u64);
        std::thread::scope(|scope| {
            for _ in 0..4 {
                let cache = cache.clone();
                scope.spawn(move || {
                    for _ in 0..1000 {
                        cache
                            .entry(1)
                            .and_upsert_with(|entry| entry.unwrap().into_value() + 1);
                    }
                });
            }
        });
        assert_eq!(cache.get(&1), Some(4000));
    }

    #[test]
    fn control_generation_before_wrap_cleans_up() {
        let cache = moka::sync::Cache::new(10);
        for value in 0..65_535 {
            cache.insert(1, value);
        }
        cache.run_pending_tasks();
        cache.invalidate_all();
        cache.run_pending_tasks();
        assert_eq!(cache.entry_count(), 0);
    }

    #[test]
    fn control_clear_unexpired_deadline_on_update() {
        struct ClearUnexpired;
        impl Expiry<u32, u32> for ClearUnexpired {
            fn expire_after_create(&self, _: &u32, _: &u32, _: Instant) -> Option<Duration> {
                Some(Duration::from_secs(60))
            }
            fn expire_after_update(
                &self,
                _: &u32,
                _: &u32,
                _: Instant,
                _: Option<Duration>,
            ) -> Option<Duration> {
                None
            }
        }
        let cache = moka::sync::Cache::builder()
            .expire_after(ClearUnexpired)
            .build();
        cache.insert(1, 0);
        cache.insert(1, 1);
        assert_eq!(cache.get(&1), Some(1));
    }

    #[test]
    fn control_remove_valid_entry() {
        let cache = moka::sync::Cache::new(10);
        cache.insert(1, 42);
        assert_eq!(cache.remove(&1), Some(42));
        assert_eq!(cache.remove(&1), None);
    }

    #[tokio::test]
    async fn control_distinct_error_type_does_not_panic() {
        let cache = moka::future::Cache::new(10);
        let (fallible, infallible) = tokio::join!(
            cache.try_get_with(1, async {
                tokio::time::sleep(Duration::from_millis(10)).await;
                Err::<u32, &str>("load failed")
            }),
            cache.get_with(1, async { 7 })
        );
        assert!(fallible.is_err());
        assert_eq!(infallible, 7);
    }
}
