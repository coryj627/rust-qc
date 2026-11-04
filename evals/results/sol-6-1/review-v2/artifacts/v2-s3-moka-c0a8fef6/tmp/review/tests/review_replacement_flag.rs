#[test]
fn review_sync_conditional_replacement_flag() {
    let cache = moka::sync::Cache::new(10);
    cache.insert("key", 10);
    let replacement = cache.entry("key").or_insert_with_if(|| 20, |_| true);
    assert_eq!(*replacement.value(), 20);
    assert_eq!(cache.get(&"key"), Some(20));
    assert!(replacement.is_old_value_replaced(), "the existing value was replaced");
}

#[tokio::test]
async fn review_future_conditional_replacement_flag() {
    let cache = moka::future::Cache::new(10);
    cache.insert("key", 10).await;
    let replacement = cache.entry("key").or_insert_with_if(async { 20 }, |_| true).await;
    assert_eq!(*replacement.value(), 20);
    assert_eq!(cache.get(&"key").await, Some(20));
    assert!(replacement.is_old_value_replaced(), "the existing value was replaced");
}
