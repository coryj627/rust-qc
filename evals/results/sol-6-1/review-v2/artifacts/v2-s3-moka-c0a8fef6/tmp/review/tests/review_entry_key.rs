#[test]
fn review_sync_fallible_borrowed_entry_keeps_key_on_hit() {
    let cache = moka::sync::Cache::new(10);
    let fresh = cache.entry_by_ref("key").or_try_insert_with(|| Ok::<u32, ()>(10)).unwrap();
    assert_eq!(fresh.key(), "key");
    let cached = cache.entry_by_ref("key").or_try_insert_with(|| Ok::<u32, ()>(20)).unwrap();
    assert_eq!(*cached.value(), 10);
    assert_eq!(cached.key(), "key");
}

#[tokio::test]
async fn review_future_fallible_borrowed_entry_keeps_key_on_hit() {
    let cache = moka::future::Cache::new(10);
    let fresh = cache.entry_by_ref("key").or_try_insert_with(async { Ok::<u32, ()>(10) }).await.unwrap();
    assert_eq!(fresh.key(), "key");
    let cached = cache.entry_by_ref("key").or_try_insert_with(async { Ok::<u32, ()>(20) }).await.unwrap();
    assert_eq!(*cached.value(), 10);
    assert_eq!(cached.key(), "key");
}
