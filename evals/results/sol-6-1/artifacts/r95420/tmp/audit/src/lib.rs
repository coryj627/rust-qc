#[cfg(test)]
mod tests {
    use std::sync::{Arc, Barrier, atomic::{AtomicBool, Ordering}};
    use std::time::Duration;
    use moka::ops::compute::Op;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn async_compute_loses_update() {
        let entered = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let should_block = Arc::new(AtomicBool::new(false));
        let cache = moka::future::Cache::builder()
            .max_capacity(100)
            .weigher({
                let entered = entered.clone();
                let release = release.clone();
                let should_block = should_block.clone();
                move |_: &u32, _: &u32| {
                    if should_block.swap(false, Ordering::SeqCst) {
                        entered.wait();
                        release.wait();
                    }
                    1
                }
            }).build();
        cache.insert(1, 0).await;
        should_block.store(true, Ordering::SeqCst);
        let c1 = cache.clone();
        let first = tokio::spawn(async move {
            c1.entry(1).and_compute_with(|e| async move {
                Op::Put(e.unwrap().into_value() + 1)
            }).await
        });
        entered.wait();
        let second = tokio::time::timeout(Duration::from_secs(2),
            cache.entry(1).and_compute_with(|e| async move {
                Op::Put(e.unwrap().into_value() + 1)
            })).await;
        release.wait();
        let first = first.await.unwrap();
        println!("first = {first:?}, second = {second:?}, final value = {:?}", cache.get(&1).await);
        assert_eq!(cache.get(&1).await, Some(2), "both increments must be retained");
    }

    #[test]
    fn generation_wrap_prevents_expiration_cleanup() {
        let cache = moka::sync::Cache::builder()
            .max_capacity(100)
            .time_to_live(Duration::from_millis(200))
            .build();
        for v in 0..65_537u32 {
            cache.insert(1, v);
            if v % 100 == 0 { cache.run_pending_tasks(); }
        }
        cache.run_pending_tasks();
        std::thread::sleep(Duration::from_millis(300));
        for _ in 0..10 { cache.run_pending_tasks(); }
        println!("after expiry: value={:?}, entry_count={}", cache.get(&1), cache.entry_count());
        assert_eq!(cache.entry_count(), 0, "expired entry must be physically removed");
    }
}

#[cfg(test)]
mod timer_tests {
    use std::sync::{Arc, Barrier, atomic::{AtomicBool, Ordering}};
    use std::time::{Duration, Instant};

    struct ClearOnRead;
    impl moka::Expiry<u32, BlockClone> for ClearOnRead {
        fn expire_after_create(&self, _: &u32, _: &BlockClone, _: Instant) -> Option<Duration> {
            Some(Duration::from_millis(500))
        }
        fn expire_after_read(&self, _: &u32, _: &BlockClone, _: Instant, _: Option<Duration>, _: Instant) -> Option<Duration> {
            None
        }
    }
    struct BlockClone {
        block: Arc<AtomicBool>,
        entered: Arc<Barrier>,
        release: Arc<Barrier>,
    }
    impl Clone for BlockClone {
        fn clone(&self) -> Self {
            if self.block.swap(false, Ordering::SeqCst) {
                self.entered.wait();
                self.release.wait();
            }
            Self { block: self.block.clone(), entered: self.entered.clone(), release: self.release.clone() }
        }
    }
    #[test]
    fn clearing_expiry_leaves_freed_timer_pointer() {
        let block = Arc::new(AtomicBool::new(false));
        let entered = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let cache = moka::sync::Cache::builder().max_capacity(100).expire_after(ClearOnRead).build();
        cache.insert(1, BlockClone { block: block.clone(), entered: entered.clone(), release: release.clone() });
        cache.run_pending_tasks();
        block.store(true, Ordering::SeqCst);
        let c = cache.clone();
        let reader = std::thread::spawn(move || { assert!(c.get(&1).is_some()); });
        entered.wait();
        // The read cleared expiration, but has not queued its read log yet.
        std::thread::sleep(Duration::from_millis(1200));
        cache.run_pending_tasks();
        // This queues the read log, whose processing tries to deschedule the freed timer.
        release.wait();
        reader.join().unwrap();
        cache.run_pending_tasks();
        cache.invalidate(&1);
        cache.run_pending_tasks();
    }
}

#[cfg(test)]
mod expiry_tests {
    use std::time::{Duration, Instant};
    struct ClearOnUpdate;
    impl moka::Expiry<u32, u32> for ClearOnUpdate {
        fn expire_after_create(&self, _: &u32, _: &u32, _: Instant) -> Option<Duration> {
            Some(Duration::from_millis(10))
        }
        fn expire_after_update(&self, _: &u32, _: &u32, _: Instant, remaining: Option<Duration>) -> Option<Duration> {
            println!("expire_after_update received {remaining:?}, returns None to clear expiry");
            None
        }
    }
    #[test]
    fn update_none_does_not_clear_expired_deadline() {
        let cache = moka::sync::Cache::builder().max_capacity(100).expire_after(ClearOnUpdate).build();
        cache.insert(1, 10);
        cache.run_pending_tasks();
        std::thread::sleep(Duration::from_millis(20));
        assert_eq!(cache.get(&1), None);
        cache.insert(1, 20);
        println!("after updating expired entry with expiry None: {:?}", cache.get(&1));
        assert_eq!(cache.get(&1), Some(20));
    }
    #[tokio::test]
    async fn async_update_none_does_not_clear_expired_deadline() {
        let cache = moka::future::Cache::builder().max_capacity(100).expire_after(ClearOnUpdate).build();
        cache.insert(1, 10).await;
        cache.run_pending_tasks().await;
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert_eq!(cache.get(&1).await, None);
        cache.insert(1, 20).await;
        println!("after updating expired entry with expiry None: {:?}", cache.get(&1).await);
        assert_eq!(cache.get(&1).await, Some(20));
    }
}

#[cfg(test)]
mod initializer_tests {
    use std::{future::{Future, poll_fn}, task::Poll};
    #[tokio::test]
    async fn get_with_panics_on_concurrent_unit_error() {
        let cache = moka::future::Cache::<u32, u32>::new(100);
        let (tx, rx) = tokio::sync::oneshot::channel();
        let mut fallible = Box::pin(cache.try_get_with(1, async {
            rx.await.unwrap();
            Err::<u32, ()>(())
        }));
        poll_fn(|cx| {
            assert!(fallible.as_mut().poll(cx).is_pending());
            Poll::Ready(())
        }).await;
        let mut infallible = Box::pin(cache.get_with(1, async { 42 }));
        poll_fn(|cx| {
            assert!(infallible.as_mut().poll(cx).is_pending());
            Poll::Ready(())
        }).await;
        tx.send(()).unwrap();
        assert!(fallible.await.is_err());
        assert_eq!(infallible.await, 42);
    }
}

#[cfg(test)]
mod weight_tests {
    use std::sync::{Arc, Barrier};
    use std::time::{Duration, Instant};
    struct PauseHeavyUpdate { entered: Arc<Barrier>, release: Arc<Barrier> }
    impl moka::Expiry<u32, u32> for PauseHeavyUpdate {
        fn expire_after_update(&self, _: &u32, value: &u32, _: Instant, _: Option<Duration>) -> Option<Duration> {
            if *value == 100 { self.entered.wait(); self.release.wait(); }
            None
        }
    }
    #[test]
    fn reordered_updates_corrupt_weighted_size() {
        let entered = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let cache = moka::sync::Cache::builder().max_capacity(1000)
            .weigher(|_: &u32, v: &u32| *v)
            .expire_after(PauseHeavyUpdate { entered: entered.clone(), release: release.clone() }).build();
        cache.insert(1, 1);
        cache.run_pending_tasks();
        let c = cache.clone();
        let heavy = std::thread::spawn(move || c.insert(1, 100));
        entered.wait();
        cache.insert(1, 1);
        cache.run_pending_tasks();
        release.wait();
        heavy.join().unwrap();
        cache.run_pending_tasks();
        println!("value={:?}, count={}, weighted_size={}", cache.get(&1), cache.entry_count(), cache.weighted_size());
        assert_eq!(cache.weighted_size(), 1);
    }
}
