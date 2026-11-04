use moka::Expiry;
use std::time::{Duration, Instant};

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
fn review_public_sync_clear_expiration() {
    let cache = moka::sync::Cache::builder().max_capacity(10).expire_after(ClearOnUpdate).build();
    cache.insert(1, 10);
    cache.insert(1, 20);
    assert_eq!(cache.get(&1), Some(20));
}

#[tokio::test]
async fn review_public_future_clear_expiration() {
    let cache = moka::future::Cache::builder().max_capacity(10).expire_after(ClearOnUpdate).build();
    cache.insert(1, 10).await;
    cache.insert(1, 20).await;
    assert_eq!(cache.get(&1).await, Some(20));
}

#[test]
fn review_public_sync_remove_after_invalidate_all() {
    let cache = moka::sync::Cache::new(10);
    cache.insert(1, 10);
    cache.invalidate_all();
    assert_eq!(cache.get(&1), None);
    assert_eq!(cache.remove(&1), None);
}

#[tokio::test]
async fn review_public_future_remove_after_invalidate_all() {
    let cache = moka::future::Cache::new(10);
    cache.insert(1, 10).await;
    cache.invalidate_all();
    assert_eq!(cache.get(&1).await, None);
    assert_eq!(cache.remove(&1).await, None);
}

#[test]
fn review_public_segmented_remove_after_invalidate_all() {
    let cache = moka::sync::SegmentedCache::new(10, 4);
    cache.insert(1, 10);
    cache.invalidate_all();
    assert_eq!(cache.get(&1), None);
    assert_eq!(cache.remove(&1), None);
}
