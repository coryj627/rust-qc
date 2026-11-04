#[cfg(test)]
mod tests {
    use futures_util::FutureExt;
    use moka::{future::Cache, notification::RemovalCause};
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        time::Duration,
    };
    use tokio::sync::{oneshot, Semaphore};

    #[tokio::test]
    async fn async_upserts_must_serialize_through_commit() {
        let gate = Arc::new(Semaphore::new(0));
        let listener_calls = Arc::new(AtomicUsize::new(0));
        let (entered_tx, entered_rx) = oneshot::channel();
        let entered_tx = Arc::new(std::sync::Mutex::new(Some(entered_tx)));
        let cache = Cache::builder()
            .max_capacity(10)
            .async_eviction_listener({
                let gate = gate.clone();
                move |_, _, cause| {
                    let gate = gate.clone();
                    let calls = listener_calls.clone();
                    let entered = entered_tx.clone();
                    async move {
                        if cause == RemovalCause::Replaced
                            && calls.fetch_add(1, Ordering::SeqCst) == 0
                        {
                            entered.lock().unwrap().take().unwrap().send(()).unwrap();
                            gate.acquire().await.unwrap().forget();
                        }
                    }
                    .boxed()
                }
            })
            .build();
        cache.insert("counter", 0u32).await;
        let a = tokio::spawn({
            let c = cache.clone();
            async move {
                c.entry("counter")
                    .and_upsert_with(|e| async move { e.unwrap().into_value() + 1 })
                    .await
            }
        });
        entered_rx.await.unwrap();
        let (b_tx, b_rx) = oneshot::channel();
        let b = tokio::spawn({
            let c = cache.clone();
            async move {
                c.entry("counter")
                    .and_upsert_with(|e| async move {
                        let n = e.unwrap().into_value();
                        b_tx.send(n).unwrap();
                        n + 1
                    })
                    .await
            }
        });
        let b_read = b_rx.await.unwrap();
        let (c_tx, c_rx) = oneshot::channel();
        let c = tokio::spawn({
            let c = cache.clone();
            async move {
                c.entry("counter")
                    .and_upsert_with(|e| async move {
                        let n = e.unwrap().into_value();
                        c_tx.send(n).unwrap();
                        n + 1
                    })
                    .await
            }
        });
        // A correct implementation waits here until the first two writes commit.
        let c_early = tokio::time::timeout(Duration::from_millis(50), c_rx).await;
        gate.add_permits(1);
        a.await.unwrap();
        b.await.unwrap();
        c.await.unwrap();
        println!(
            "second read: {b_read}; third early read: {c_early:?}; final: {:?}",
            cache.get(&"counter").await
        );
        assert_eq!(cache.get(&"counter").await, Some(3));
    }

    struct CreationExpiry;
    impl moka::Expiry<&'static str, u32> for CreationExpiry {
        fn expire_after_create(
            &self,
            _: &&'static str,
            _: &u32,
            _: std::time::Instant,
        ) -> Option<Duration> {
            Some(Duration::from_millis(30))
        }
        fn expire_after_update(
            &self,
            _: &&'static str,
            _: &u32,
            _: std::time::Instant,
            _: Option<Duration>,
        ) -> Option<Duration> {
            None // Explicitly clear expiration, as documented.
        }
    }

    #[tokio::test]
    async fn async_clear_expired_deadline() {
        let cache = Cache::builder()
            .max_capacity(10)
            .expire_after(CreationExpiry)
            .build();
        cache.insert("key", 1).await;
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(cache.get(&"key").await, None);
        cache.insert("key", 2).await;
        assert_eq!(cache.get(&"key").await, Some(2));
    }

    #[test]
    fn clear_unexpired_deadline_control() {
        let cache = moka::sync::Cache::builder()
            .max_capacity(10)
            .expire_after(CreationExpiry)
            .build();
        cache.insert("key", 1);
        cache.insert("key", 2);
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(cache.get(&"key"), Some(2));
    }

    #[test]
    fn reinsert_after_expiry() {
        let cache = moka::sync::Cache::builder()
            .max_capacity(10)
            .expire_after(CreationExpiry)
            .build();
        cache.insert("key", 1);
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(cache.get(&"key"), None);
        cache.insert("key", 2);
        println!("reinserted value: {:?}", cache.get(&"key"));
        assert_eq!(cache.get(&"key"), Some(2));
    }
    #[test]
    fn sync_error_and_optional_initialization_controls() {
        let cache = moka::sync::Cache::<String, u32>::new(10);
        let err = cache
            .try_get_with("key".into(), || Err::<u32, _>("load failed"))
            .unwrap_err();
        assert_eq!(*err, "load failed");
        assert_eq!(cache.get("key"), None);
        assert_eq!(cache.optionally_get_with("key".into(), || None), None);
        assert_eq!(cache.get_with("key".into(), || 7), 7);
        let result = cache
            .entry_by_ref("key")
            .and_try_compute_with(|_| Err::<moka::ops::compute::Op<u32>, _>("compute failed"));
        assert_eq!(result.unwrap_err(), "compute failed");
        assert_eq!(cache.get("key"), Some(7));
        assert!(matches!(
            cache.invalidate_entries_if(|_, _| true),
            Err(moka::PredicateError::InvalidationClosuresDisabled)
        ));
    }

    #[tokio::test]
    async fn async_error_and_optional_initialization_controls() {
        let cache = Cache::<String, u32>::new(10);
        let err = cache
            .try_get_with("key".into(), async { Err::<u32, _>("load failed") })
            .await
            .unwrap_err();
        assert_eq!(*err, "load failed");
        assert_eq!(cache.get("key").await, None);
        assert_eq!(
            cache
                .optionally_get_with("key".into(), async { None })
                .await,
            None
        );
        assert_eq!(cache.get_with("key".into(), async { 7 }).await, 7);
        let result = cache
            .entry_by_ref("key")
            .and_try_compute_with(|_| async {
                Err::<moka::ops::compute::Op<u32>, _>("compute failed")
            })
            .await;
        assert_eq!(result.unwrap_err(), "compute failed");
        assert_eq!(cache.get("key").await, Some(7));
        assert!(matches!(
            cache.invalidate_entries_if(|_, _| true),
            Err(moka::PredicateError::InvalidationClosuresDisabled)
        ));
    }
}
