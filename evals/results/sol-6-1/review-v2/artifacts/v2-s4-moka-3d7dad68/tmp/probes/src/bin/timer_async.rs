use moka::Expiry;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Notify;
struct Exp;
impl Expiry<u32, u32> for Exp {
    fn expire_after_create(&self, k: &u32, _: &u32, _: Instant) -> Option<Duration> {
        (*k == 0).then_some(Duration::from_millis(1))
    }
    fn expire_after_read(
        &self,
        k: &u32,
        _: &u32,
        _: Instant,
        d: Option<Duration>,
        _: Instant,
    ) -> Option<Duration> {
        if *k == 1 {
            d.or(Some(Duration::from_millis(40)))
        } else {
            d
        }
    }
}
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let started = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let c = moka::future::Cache::builder()
        .expire_after(Exp)
        .async_eviction_listener({
            let started = started.clone();
            let release = release.clone();
            move |k, _, _| {
                let started = started.clone();
                let release = release.clone();
                Box::pin(async move {
                    if *k == 0 {
                        started.notify_one();
                        release.notified().await;
                    }
                })
            }
        })
        .build();
    c.insert(0, 0).await;
    c.insert(1, 1).await;
    c.insert(2, 2).await;
    c.run_pending_tasks().await;
    tokio::time::sleep(Duration::from_millis(1200)).await;
    let maintenance = tokio::spawn({
        let c = c.clone();
        async move { c.run_pending_tasks().await }
    });
    tokio::time::timeout(Duration::from_secs(3), started.notified())
        .await
        .unwrap();
    let count = if std::env::args().any(|a| a == "--control") {
        0
    } else {
        1000
    };
    for _ in 0..count {
        assert_eq!(c.get(&2).await, Some(2));
    }
    assert_eq!(c.get(&1).await, Some(1));
    release.notify_one();
    maintenance.await.unwrap();
    tokio::time::sleep(Duration::from_millis(1300)).await;
    for _ in 0..5 {
        c.run_pending_tasks().await;
    }
    assert_eq!(c.get(&1).await, None);
    let maintained_count = c.entry_count();
    let visible_count = c.iter().count();
    let retained = c.remove(&1).await;
    println!("visible entries: {visible_count}; maintained entry_count: {maintained_count}; removed retained value: {retained:?}");
    assert_eq!(
        maintained_count, 1,
        "expired entry should have been removed by maintenance"
    );
}
