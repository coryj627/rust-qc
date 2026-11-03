// Async writers blocked on a full write-op channel: lost wakeup stress. Prints if a run stalls.
use moka::future::Cache;
use std::time::Duration;
#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let mut stalls = 0;
    for round in 0..150 {
        let cache: Cache<u64, u64> = Cache::builder().max_capacity(100).eviction_listener(|_, _, _| {
            std::thread::sleep(Duration::from_micros(20));
        }).build();
        let mut hs = vec![];
        for t in 0..64u64 {
            let c = cache.clone();
            hs.push(tokio::spawn(async move {
                for i in 0..200u64 { c.insert(t * 1000 + i, i).await; }
            }));
        }
        let all = futures_util::future::join_all(hs);
        match tokio::time::timeout(Duration::from_secs(15), all).await {
            Ok(_) => {}
            Err(_) => { stalls += 1; println!("round {round}: writers stalled for 15s, ch? entry_count={}", cache.entry_count()); break; }
        }
    }
    println!("stalls: {stalls}");
    std::process::exit(0);
}
