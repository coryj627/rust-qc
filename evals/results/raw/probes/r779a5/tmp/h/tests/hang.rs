use moka::future::Cache;
use std::time::Duration;
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn lost_wakeup_probe() {
    let start = std::time::Instant::now();
    let mut round = 0u32;
    while start.elapsed() < Duration::from_secs(150) {
        round += 1;
        let c: Cache<u32, u32> = Cache::builder()
            .async_eviction_listener(|_, _, _| Box::pin(async {}))
            .max_capacity(50).build();
        let mut hs = vec![];
        for t in 0..800u32 {
            let c = c.clone();
            hs.push(tokio::spawn(async move { c.insert(t, t).await; }));
        }
        let r = tokio::time::timeout(Duration::from_secs(5), async move { for h in hs { h.await.unwrap(); } }).await;
        assert!(r.is_ok(), "writers hung at round {round}");
    }
    println!("no hang in {round} rounds");
}
