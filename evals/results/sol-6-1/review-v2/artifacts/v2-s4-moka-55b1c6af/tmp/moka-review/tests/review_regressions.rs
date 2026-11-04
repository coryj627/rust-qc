use std::sync::{Arc, Mutex, Condvar, atomic::{AtomicBool, Ordering}, mpsc};
use std::time::Duration;

#[test]
fn future_upserts_must_serialize_through_write() {
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
    let (entered_tx, entered_rx) = mpsc::channel();
    let release = Arc::new((Mutex::new(false), Condvar::new()));
    let release2 = release.clone();
    let first = AtomicBool::new(true);
    let cache = moka::future::Cache::builder().max_capacity(100)
        .weigher(move |_: &u32, value: &u32| {
            if *value == 1 && first.swap(false, Ordering::SeqCst) {
                entered_tx.send(()).unwrap();
                let (lock, cv) = &*release2;
                let mut go = lock.lock().unwrap();
                while !*go { go = cv.wait(go).unwrap(); }
            }
            1
        }).build();
    rt.block_on(cache.insert(7, 0));
    let c1 = cache.clone();
    let task1 = rt.spawn(async move {
        c1.entry(7).and_upsert_with(|e| std::future::ready(e.unwrap().into_value() + 1)).await
    });
    entered_rx.recv_timeout(Duration::from_secs(3)).unwrap();
    let (observed_tx, observed_rx) = mpsc::channel();
    let c2 = cache.clone();
    let task2 = rt.spawn(async move {
        c2.entry(7).and_upsert_with(|e| {
            let old = e.unwrap().into_value();
            observed_tx.send(old).unwrap();
            std::future::ready(old + 1)
        }).await
    });
    let observed = observed_rx.recv_timeout(Duration::from_millis(300));
    {
        let (lock, cv) = &*release;
        *lock.lock().unwrap() = true;
        cv.notify_all();
    }
    rt.block_on(async { task1.await.unwrap(); task2.await.unwrap(); });
    let final_value = rt.block_on(cache.get(&7));
    println!("second computation observed {observed:?}; final value {final_value:?}");
    assert_eq!(final_value, Some(2), "serialized increments must both apply");
}

#[test]
fn sync_get_with_can_retry_after_weigher_panic() {
    let first = AtomicBool::new(true);
    let cache = moka::sync::Cache::builder().max_capacity(100)
        .weigher(move |_: &u32, _: &u32| {
            if first.swap(false, Ordering::SeqCst) { panic!("one-time callback failure"); }
            1
        }).build();
    let worker_cache = cache.clone();
    let initial = std::thread::spawn(move || worker_cache.get_with(7, || 1)).join();
    assert!(initial.is_err());
    assert_eq!(cache.get_with(7, || 2), 2);
}

struct ClearOnUpdate;
impl moka::Expiry<u32, u32> for ClearOnUpdate {
    fn expire_after_create(&self, _: &u32, _: &u32, _: std::time::Instant) -> Option<Duration> {
        Some(Duration::ZERO)
    }
    fn expire_after_update(&self, _: &u32, _: &u32, _: std::time::Instant, remaining: Option<Duration>) -> Option<Duration> {
        println!("expiry update remaining: {remaining:?}; return None to clear expiration");
        None
    }
}

#[test]
fn sync_update_clears_elapsed_custom_expiration() {
    let cache = moka::sync::Cache::builder().max_capacity(100).expire_after(ClearOnUpdate).build();
    cache.insert(7, 1);
    assert_eq!(cache.get(&7), None);
    cache.insert(7, 2);
    assert_eq!(cache.get(&7), Some(2));
}

#[tokio::test]
async fn future_update_clears_elapsed_custom_expiration() {
    let cache = moka::future::Cache::builder().max_capacity(100).expire_after(ClearOnUpdate).build();
    cache.insert(7, 1).await;
    assert_eq!(cache.get(&7).await, None);
    cache.insert(7, 2).await;
    assert_eq!(cache.get(&7).await, Some(2));
}

#[test]
fn sync_borrowed_try_entry_retains_key_on_hit() {
    let cache = moka::sync::Cache::<String, u32>::new(100);
    cache.insert("key".to_owned(), 1);
    let entry = cache.entry_by_ref("key").or_try_insert_with(|| Ok::<_, ()>(2)).unwrap();
    assert_eq!(entry.value(), &1);
    assert_eq!(entry.key(), "key");
}

#[test]
fn segmented_borrowed_try_entry_retains_key_on_hit() {
    let cache = moka::sync::SegmentedCache::<String, u32>::new(100, 4);
    cache.insert("key".to_owned(), 1);
    let entry = cache.entry_by_ref("key").or_try_insert_with(|| Ok::<_, ()>(2)).unwrap();
    assert_eq!(entry.value(), &1);
    assert_eq!(entry.key(), "key");
}

#[tokio::test]
async fn future_borrowed_try_entry_retains_key_on_hit_control() {
    let cache = moka::future::Cache::<String, u32>::new(100);
    cache.insert("key".to_owned(), 1).await;
    let entry = cache.entry_by_ref("key").or_try_insert_with(async { Ok::<_, ()>(2) }).await.unwrap();
    assert_eq!(entry.value(), &1);
    assert_eq!(entry.key(), "key");
}

#[tokio::test]
async fn future_get_with_can_retry_after_weigher_panic_control() {
    let first = AtomicBool::new(true);
    let cache = moka::future::Cache::builder().max_capacity(100)
        .weigher(move |_: &u32, _: &u32| {
            if first.swap(false, Ordering::SeqCst) { panic!("one-time callback failure"); }
            1
        }).build();
    let worker_cache = cache.clone();
    assert!(tokio::spawn(async move { worker_cache.get_with(7, async { 1 }).await }).await.is_err());
    assert_eq!(cache.get_with(7, async { 2 }).await, 2);
}
