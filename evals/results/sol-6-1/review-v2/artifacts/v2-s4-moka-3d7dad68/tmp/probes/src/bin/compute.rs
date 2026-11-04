use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let entered = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let c = moka::future::Cache::builder()
        .async_eviction_listener({
            let entered = entered.clone();
            let release = release.clone();
            move |_, old: u32, _| {
                let entered = entered.clone();
                let release = release.clone();
                Box::pin(async move {
                    if old == 0 {
                        entered.notify_one();
                        release.notified().await;
                    }
                })
            }
        })
        .build();
    c.insert(1, 0).await;
    let blocker = tokio::spawn({
        let c = c.clone();
        async move { c.insert(1, 100).await }
    });
    entered.notified().await;
    let seen = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let compute = || {
        let c = c.clone();
        let seen = seen.clone();
        tokio::spawn(async move {
            c.entry(1)
                .and_upsert_with(|entry| {
                    let v = entry.unwrap().into_value();
                    println!("compute read {v}");
                    seen.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    std::future::ready(v + 1)
                })
                .await
                .into_value()
        })
    };
    let a = compute();
    let b = compute();
    tokio::time::timeout(Duration::from_secs(2), async {
        while seen.load(std::sync::atomic::Ordering::SeqCst) < 2 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("both computations reached write stage");
    release.notify_one();
    blocker.await.unwrap();
    println!(
        "returned {:?}, {:?}; final {:?}",
        a.await.unwrap(),
        b.await.unwrap(),
        c.get(&1).await
    );
    assert_eq!(
        c.get(&1).await,
        Some(102),
        "serialized increments must both persist"
    );
}
