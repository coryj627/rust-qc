use moka::future::Cache;

#[tokio::test]
async fn review_get_with_coexists_with_unit_error() {
    let cache = Cache::new(10);
    let (result, value) = tokio::join!(
        cache.try_get_with(1, async {
            tokio::task::yield_now().await;
            Err::<u32, ()>(())
        }),
        cache.get_with(1, async { 42u32 })
    );
    assert!(result.is_err());
    assert_eq!(value, 42);
    assert_eq!(cache.get(&1).await, Some(42));
}

#[test]
fn review_sync_get_with_coexists_with_unit_error() {
    use std::sync::mpsc;
    let cache = moka::sync::Cache::new(10);
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let first_cache = cache.clone();
    let first = std::thread::spawn(move || first_cache.try_get_with(1, || {
        started_tx.send(()).unwrap();
        release_rx.recv().unwrap();
        Err::<u32, ()>(())
    }));
    started_rx.recv().unwrap();
    let (second_started_tx, second_started_rx) = mpsc::channel();
    let second_cache = cache.clone();
    let second = std::thread::spawn(move || {
        second_started_tx.send(()).unwrap();
        second_cache.get_with(1, || 42u32)
    });
    second_started_rx.recv().unwrap();
    // Let the second caller reach the in-flight initialization before releasing it.
    std::thread::sleep(std::time::Duration::from_millis(30));
    release_tx.send(()).unwrap();
    assert!(first.join().unwrap().is_err());
    assert_eq!(second.join().expect("get_with should handle another initializer's failure"), 42);
}
