use moka::policy::EvictionPolicy;

#[test]
fn sync_lru_after_generation_wrap() {
    let cache = moka::sync::Cache::builder()
        .max_capacity(1)
        .eviction_policy(EvictionPolicy::lru())
        .build();
    for value in 0..65_536u32 {
        cache.insert("hot", value);
        cache.run_pending_tasks();
    }
    cache.insert("new", 1);
    cache.run_pending_tasks();
    assert_eq!(cache.get(&"hot"), None, "LRU entry must be evicted");
    assert_eq!(cache.get(&"new"), Some(1));
}

#[tokio::test]
async fn future_lru_after_generation_wrap() {
    let cache = moka::future::Cache::builder()
        .max_capacity(1)
        .eviction_policy(EvictionPolicy::lru())
        .build();
    for value in 0..65_536u32 {
        cache.insert("hot", value).await;
        cache.run_pending_tasks().await;
    }
    cache.insert("new", 1).await;
    cache.run_pending_tasks().await;
    assert_eq!(cache.get(&"hot").await, None, "LRU entry must be evicted");
    assert_eq!(cache.get(&"new").await, Some(1));
}

use std::sync::{Arc, Barrier, mpsc};
use std::time::{Duration, Instant};

struct PauseSecondUpdate {
    entered: mpsc::Sender<()>,
    release: Arc<Barrier>,
}
impl moka::Expiry<&'static str, u32> for PauseSecondUpdate {
    fn expire_after_update(&self, _: &&'static str, value: &u32, _: Instant, duration: Option<Duration>) -> Option<Duration> {
        if *value == 2 {
            self.entered.send(()).unwrap();
            self.release.wait();
        }
        duration
    }
}

#[test]
fn sync_out_of_order_weight_updates() {
    let (entered_tx, entered_rx) = mpsc::channel();
    let release = Arc::new(Barrier::new(2));
    let cache = moka::sync::Cache::builder()
        .max_capacity(100)
        .weigher(|_, value: &u32| *value)
        .expire_after(PauseSecondUpdate { entered: entered_tx, release: release.clone() })
        .build();
    cache.insert("key", 1);
    cache.run_pending_tasks();
    let worker = {
        let cache = cache.clone();
        std::thread::spawn(move || cache.insert("key", 2))
    };
    entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    cache.insert("key", 3);
    release.wait();
    worker.join().unwrap();
    cache.run_pending_tasks();
    assert_eq!(cache.get(&"key"), Some(3));
    assert_eq!(cache.entry_count(), 1);
    assert_eq!(cache.weighted_size(), 3, "weight must match the only cached value");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn future_out_of_order_weight_updates() {
    let (entered_tx, entered_rx) = mpsc::channel();
    let release = Arc::new(Barrier::new(2));
    let cache = moka::future::Cache::builder()
        .max_capacity(100)
        .weigher(|_, value: &u32| *value)
        .expire_after(PauseSecondUpdate { entered: entered_tx, release: release.clone() })
        .build();
    cache.insert("key", 1).await;
    cache.run_pending_tasks().await;
    let worker = {
        let cache = cache.clone();
        tokio::spawn(async move { cache.insert("key", 2).await; })
    };
    entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    cache.insert("key", 3).await;
    release.wait();
    worker.await.unwrap();
    cache.run_pending_tasks().await;
    assert_eq!(cache.get(&"key").await, Some(3));
    assert_eq!(cache.entry_count(), 1);
    assert_eq!(cache.weighted_size(), 3, "weight must match the only cached value");
}

struct ClearExpiryOnUpdate(Arc<std::sync::atomic::AtomicUsize>);
impl moka::Expiry<&'static str, u32> for ClearExpiryOnUpdate {
    fn expire_after_create(&self, _: &&'static str, _: &u32, _: Instant) -> Option<Duration> {
        Some(Duration::from_millis(10))
    }
    fn expire_after_update(&self, _: &&'static str, _: &u32, _: Instant, _: Option<Duration>) -> Option<Duration> {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        None // Documented to clear expiration.
    }
}

#[test]
fn sync_clear_elapsed_expiry_on_update() {
    let updates = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let cache = moka::sync::Cache::builder()
        .expire_after(ClearExpiryOnUpdate(updates.clone()))
        .build();
    cache.insert("key", 1);
    std::thread::sleep(Duration::from_millis(30));
    assert_eq!(cache.get(&"key"), None);
    cache.insert("key", 2);
    assert_eq!(updates.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(cache.get(&"key"), Some(2), "update returning None must clear expiration");
}

#[tokio::test]
async fn future_clear_elapsed_expiry_on_update() {
    let updates = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let cache = moka::future::Cache::builder()
        .expire_after(ClearExpiryOnUpdate(updates.clone()))
        .build();
    cache.insert("key", 1).await;
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert_eq!(cache.get(&"key").await, None);
    cache.insert("key", 2).await;
    assert_eq!(updates.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(cache.get(&"key").await, Some(2), "update returning None must clear expiration");
}

#[test]
fn sync_out_of_order_weight_updates_evict_fitting_value() {
    let (entered_tx, entered_rx) = mpsc::channel();
    let release = Arc::new(Barrier::new(2));
    let cache = moka::sync::Cache::builder()
        .max_capacity(3)
        .weigher(|_, value: &u32| *value)
        .expire_after(PauseSecondUpdate { entered: entered_tx, release: release.clone() })
        .build();
    cache.insert("key", 1);
    cache.run_pending_tasks();
    let worker = {
        let cache = cache.clone();
        std::thread::spawn(move || cache.insert("key", 2))
    };
    entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    cache.insert("key", 3);
    release.wait();
    worker.join().unwrap();
    cache.run_pending_tasks();
    eprintln!("after reordered updates: value={:?}, count={}, weight={}", cache.get(&"key"), cache.entry_count(), cache.weighted_size());
    cache.insert("next", 3);
    cache.run_pending_tasks();
    eprintln!("after fresh insertion: next={:?}, count={}, weight={}", cache.get(&"next"), cache.entry_count(), cache.weighted_size());
    assert_eq!(cache.get(&"next"), Some(3), "an empty cache must admit an entry fitting its capacity");
}

#[tokio::test]
async fn future_compute_keeps_lock_until_write_finishes() {
    use futures_util::{FutureExt, pin_mut, poll, join};
    use moka::{notification::RemovalCause, ops::compute::Op};
    let release = Arc::new(tokio::sync::Notify::new());
    let release_for_listener = release.clone();
    let cache = moka::future::Cache::builder()
        .max_capacity(100)
        .async_eviction_listener(move |_, old_value, cause| {
            let release = release_for_listener.clone();
            async move {
                if old_value == 0 && cause == RemovalCause::Replaced {
                    release.notified().await;
                }
            }.boxed()
        })
        .build();
    cache.insert("counter", 0u32).await;
    let first = cache.entry("counter").and_compute_with(|entry| async move {
        Op::Put(entry.unwrap().into_value() + 1)
    });
    pin_mut!(first);
    assert!(poll!(first.as_mut()).is_pending());
    let second = cache.entry("counter").and_compute_with(|entry| async move {
        Op::Put(entry.unwrap().into_value() + 1)
    });
    pin_mut!(second);
    assert!(poll!(second.as_mut()).is_pending());
    let third = cache.entry("counter").and_compute_with(|entry| async move {
        Op::Put(entry.unwrap().into_value() + 1)
    });
    pin_mut!(third);
    assert!(poll!(third.as_mut()).is_pending());
    release.notify_one();
    let results = tokio::time::timeout(Duration::from_secs(5), async { join!(first, second, third) }).await.unwrap();
    eprintln!("compute results: {results:?}");
    assert_eq!(cache.get(&"counter").await, Some(3), "three serialized increments must retain all updates");
}

#[test]
fn sync_borrowed_try_entry_hit_has_key() {
    let cache = moka::sync::Cache::new(100);
    cache.insert("key".to_string(), 42u32);
    let entry = cache.entry_by_ref("key").or_try_insert_with(|| Ok::<_, ()>(99)).unwrap();
    assert_eq!(entry.value(), &42);
    assert_eq!(entry.key(), "key");
}

#[test]
fn segmented_borrowed_try_entry_hit_has_key() {
    let cache = moka::sync::SegmentedCache::new(100, 4);
    cache.insert("key".to_string(), 42u32);
    let entry = cache.entry_by_ref("key").or_try_insert_with(|| Ok::<_, ()>(99)).unwrap();
    assert_eq!(entry.value(), &42);
    assert_eq!(entry.key(), "key");
}

#[tokio::test]
async fn future_borrowed_try_entry_hit_has_key() {
    let cache = moka::future::Cache::new(100);
    cache.insert("key".to_string(), 42u32).await;
    let entry = cache.entry_by_ref("key").or_try_insert_with(async { Ok::<_, ()>(99) }).await.unwrap();
    assert_eq!(entry.value(), &42);
    assert_eq!(entry.key(), "key");
}
