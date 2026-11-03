// future::Cache and_upsert_with / and_compute_with used as an atomic counter.
use moka::future::Cache;
use moka::ops::compute::Op;
#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    for round in 0..5 {
        let cache: Cache<u32, u64> = Cache::new(100);
        let mut hs = vec![];
        for _ in 0..16 {
            let c = cache.clone();
            hs.push(tokio::spawn(async move {
                for _ in 0..2000 {
                    c.entry(1).and_upsert_with(|e| async move { e.map(|e| e.into_value() + 1).unwrap_or(1) }).await;
                }
            }));
        }
        for h in hs { h.await.unwrap(); }
        println!("round {round}: and_upsert_with expected 32000 got {:?}", cache.get(&1).await);
    }
    let cache: Cache<u32, u64> = Cache::new(100);
    let mut hs = vec![];
    for _ in 0..16 {
        let c = cache.clone();
        hs.push(tokio::spawn(async move {
            for _ in 0..2000 {
                c.entry(1).and_compute_with(|e| async move { Op::Put(e.map(|e| e.into_value() + 1).unwrap_or(1)) }).await;
            }
        }));
    }
    for h in hs { h.await.unwrap(); }
    println!("and_compute_with expected 32000 got {:?}", cache.get(&1).await);
}
