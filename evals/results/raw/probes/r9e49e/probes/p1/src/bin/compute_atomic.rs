use moka::ops::compute::Op;
#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let c: moka::future::Cache<u32, u64> = moka::future::Cache::new(100);
    let mut hs = vec![];
    for _ in 0..8 {
        let c = c.clone();
        hs.push(tokio::spawn(async move {
            for _ in 0..100_000u32 {
                c.entry(1).and_compute_with(|e| { let v = e.map(|e| e.into_value()).unwrap_or(0); std::future::ready(Op::Put(v + 1)) }).await;
            }
        }));
    }
    for h in hs { h.await.unwrap(); }
    println!("future and_compute_with +1, 800000 total: value={:?}", c.get(&1).await);

    let c: moka::future::Cache<u32, u64> = moka::future::Cache::new(100);
    let mut hs = vec![];
    for _ in 0..8 {
        let c = c.clone();
        hs.push(tokio::spawn(async move {
            for _ in 0..100_000u32 {
                c.entry(1).and_upsert_with(|e| { let v = e.map(|e| e.into_value()).unwrap_or(0); std::future::ready(v + 1) }).await;
            }
        }));
    }
    for h in hs { h.await.unwrap(); }
    println!("future and_upsert_with +1, 800000 total: value={:?}", c.get(&1).await);

    let c: moka::sync::Cache<u32, u64> = moka::sync::Cache::new(100);
    let mut hs = vec![];
    for _ in 0..8 {
        let c = c.clone();
        hs.push(std::thread::spawn(move || {
            for _ in 0..100_000u32 {
                c.entry(1).and_compute_with(|e| { let v = e.map(|e| e.into_value()).unwrap_or(0); Op::Put(v + 1) });
            }
        }));
    }
    for h in hs { h.join().unwrap(); }
    println!("sync and_compute_with +1, 800000 total: value={:?}", c.get(&1));
}
