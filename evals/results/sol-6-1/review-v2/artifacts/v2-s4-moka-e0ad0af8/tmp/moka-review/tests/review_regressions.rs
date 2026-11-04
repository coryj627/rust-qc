use moka::Expiry;
use std::{sync::{Arc, atomic::{AtomicBool, Ordering}}, time::{Duration, Instant}};

struct ClearOnUpdate;
impl Expiry<u32, u32> for ClearOnUpdate {
    fn expire_after_create(&self, _: &u32, _: &u32, _: Instant) -> Option<Duration> {
        Some(Duration::ZERO)
    }
    fn expire_after_update(&self, _: &u32, _: &u32, _: Instant, _: Option<Duration>) -> Option<Duration> {
        None
    }
}

#[test]
fn sync_update_none_clears_expiration() {
    let cache = moka::sync::Cache::builder().expire_after(ClearOnUpdate).build();
    cache.insert(1, 10);
    assert_eq!(cache.get(&1), None);
    cache.insert(1, 20);
    assert_eq!(cache.get(&1), Some(20), "expire_after_update returned None, which should clear expiration");
}

#[tokio::test]
async fn future_update_none_clears_expiration() {
    let cache = moka::future::Cache::builder().expire_after(ClearOnUpdate).build();
    cache.insert(1, 10).await;
    assert_eq!(cache.get(&1).await, None);
    cache.insert(1, 20).await;
    assert_eq!(cache.get(&1).await, Some(20), "expire_after_update returned None, which should clear expiration");
}

#[test]
fn sync_get_with_recovers_after_weigher_panic() {
    let first = Arc::new(AtomicBool::new(true));
    let flag = first.clone();
    let cache = moka::sync::Cache::builder().max_capacity(10)
        .weigher(move |_: &u32, _: &u32| {
            if flag.swap(false, Ordering::SeqCst) { panic!("one-off weigher panic"); }
            1
        }).build();
    let worker_cache = cache.clone();
    assert!(std::thread::spawn(move || worker_cache.get_with(1, || 10)).join().is_err());
    assert_eq!(cache.get_with(1, || 20), 20);
}

#[tokio::test]
async fn future_infallible_get_with_does_not_receive_unit_error() {
    let cache = moka::future::Cache::new(10);
    let (send, receive) = tokio::sync::oneshot::channel();
    let mut fallible = Box::pin(cache.try_get_with(1, async { receive.await.unwrap() }));
    assert!(futures_util::poll!(fallible.as_mut()).is_pending());
    let mut infallible = Box::pin(cache.get_with(1, async { 20u32 }));
    assert!(futures_util::poll!(infallible.as_mut()).is_pending());
    send.send(Err::<u32, ()>(())).unwrap();
    assert_eq!(fallible.await, Err(Arc::new(())));
    assert_eq!(infallible.await, 20);
}

#[test]
fn sync_lru_survives_generation_wrap() {
    let cache = moka::sync::Cache::builder().max_capacity(1)
        .eviction_policy(moka::policy::EvictionPolicy::lru()).build();
    for value in 0..=u16::MAX as u32 {
        cache.insert(1, value);
        cache.run_pending_tasks();
    }
    cache.insert(2, 99);
    cache.run_pending_tasks();
    assert_eq!((cache.get(&1), cache.get(&2)), (None, Some(99)), "LRU should retain the newer key");
}

#[tokio::test]
async fn future_lru_survives_generation_wrap() {
    let cache = moka::future::Cache::builder().max_capacity(1)
        .eviction_policy(moka::policy::EvictionPolicy::lru()).build();
    for value in 0..=u16::MAX as u32 {
        cache.insert(1, value).await;
        cache.run_pending_tasks().await;
    }
    cache.insert(2, 99).await;
    cache.run_pending_tasks().await;
    assert_eq!((cache.get(&1).await, cache.get(&2).await), (None, Some(99)), "LRU should retain the newer key");
}

#[tokio::test]
async fn future_upserts_hold_serialization_until_write() {
    use futures_util::FutureExt;
    let gate = Arc::new(tokio::sync::Notify::new());
    let listener_gate = gate.clone();
    let cache = moka::future::Cache::builder().max_capacity(10)
        .async_eviction_listener(move |_, value, _| {
            let gate = listener_gate.clone();
            async move { if value == 0u32 { gate.notified().await; } }.boxed()
        }).build();
    cache.insert(1, 0u32).await;
    let mut previous_write = Box::pin(cache.insert(1, 1));
    assert!(futures_util::poll!(previous_write.as_mut()).is_pending());
    assert_eq!(cache.get(&1).await, Some(1));
    let mut first = Box::pin(cache.entry(1).and_upsert_with(|entry| async move {
        entry.unwrap().into_value() + 1
    }));
    assert!(futures_util::poll!(first.as_mut()).is_pending());
    let mut second = Box::pin(cache.entry(1).and_upsert_with(|entry| async move {
        entry.unwrap().into_value() + 1
    }));
    assert!(futures_util::poll!(second.as_mut()).is_pending());
    gate.notify_one();
    previous_write.await;
    first.await;
    second.await;
    assert_eq!(cache.get(&1).await, Some(3), "serialized increments must both be applied");
}

#[test]
fn sync_lru_control_before_generation_wrap() {
    let cache = moka::sync::Cache::builder().max_capacity(1)
        .eviction_policy(moka::policy::EvictionPolicy::lru()).build();
    for value in 0..u16::MAX as u32 {
        cache.insert(1, value);
        cache.run_pending_tasks();
    }
    cache.insert(2, 99);
    cache.run_pending_tasks();
    assert_eq!((cache.get(&1), cache.get(&2)), (None, Some(99)));
}

#[test]
fn sync_infallible_get_with_does_not_receive_unit_error() {
    use std::{hash::{Hash, Hasher}, sync::mpsc};
    #[derive(Clone)]
    struct Key(mpsc::Sender<()>);
    impl Hash for Key { fn hash<H: Hasher>(&self, state: &mut H) { 1u32.hash(state); } }
    impl PartialEq for Key {
        fn eq(&self, _: &Self) -> bool { self.0.send(()).unwrap(); true }
    }
    impl Eq for Key {}
    let (matched_send, matched_receive) = mpsc::channel();
    let (started_send, started_receive) = mpsc::channel();
    let (release_send, release_receive) = mpsc::channel();
    let cache = moka::sync::Cache::new(10);
    let first_cache = cache.clone();
    let first_key = Key(matched_send.clone());
    let first = std::thread::spawn(move || first_cache.try_get_with(first_key, || {
        started_send.send(()).unwrap();
        release_receive.recv().unwrap();
        Err::<u32, ()>(())
    }));
    started_receive.recv_timeout(Duration::from_secs(2)).unwrap();
    let second_cache = cache.clone();
    let second = std::thread::spawn(move || second_cache.get_with(Key(matched_send), || 20u32));
    // Matching the waiter's key confirms the second caller is joining the first.
    matched_receive.recv_timeout(Duration::from_secs(2)).unwrap();
    release_send.send(()).unwrap();
    assert_eq!(first.join().unwrap(), Err(Arc::new(())));
    assert_eq!(second.join().unwrap(), 20);
}
