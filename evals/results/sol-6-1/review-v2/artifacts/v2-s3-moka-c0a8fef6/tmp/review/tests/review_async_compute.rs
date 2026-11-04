#![cfg(feature = "future")]
use futures_util::{FutureExt, future::join_all};
use moka::future::Cache;

#[tokio::test]
async fn review_concurrent_upserts_with_async_listener() {
    let cache = Cache::builder()
        .max_capacity(100)
        .async_eviction_listener(|_, _, _| async { tokio::task::yield_now().await }.boxed())
        .build();
    cache.insert(1, 0u32).await;
    join_all((0..10).map(|_| cache.entry(1).and_upsert_with(|entry| async move {
        entry.map_or(1, |e| e.into_value() + 1)
    }))).await;
    assert_eq!(cache.get(&1).await, Some(10), "each serialized upsert must see the preceding update");
}
