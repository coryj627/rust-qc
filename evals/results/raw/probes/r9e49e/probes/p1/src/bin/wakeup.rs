// Do writers parked on a full write-op channel always get woken?
use moka::future::Cache;
use std::time::{Duration, Instant};
#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let start = Instant::now();
    let mut rounds = 0u32;
    while start.elapsed() < Duration::from_secs(90) {
        rounds += 1;
        let c: Cache<u32, u32> = Cache::builder().max_capacity(100_000)
            .async_eviction_listener(|_, _, _| Box::pin(async { tokio::task::yield_now().await })).build();
        let mut hs = vec![];
        for t in 0..64u32 {
            let c = c.clone();
            hs.push(tokio::spawn(async move { for i in 0..400u32 { c.insert(i + t * 1000, i).await; } }));
        }
        let all = futures_util::future::join_all(hs);
        if tokio::time::timeout(Duration::from_secs(15), all).await.is_err() {
            println!("HANG in round {rounds}: writers still parked after 15 s, entry_count={}", c.entry_count());
            std::process::exit(0);
        }
    }
    println!("no hang in {rounds} rounds");
    std::process::exit(0);
}
