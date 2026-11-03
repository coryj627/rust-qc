#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let c: moka::future::Cache<u32, u32> = moka::future::Cache::builder().max_capacity(1000).build();
    let mut hs = vec![];
    for t in 0..4u32 {
        let c = c.clone();
        hs.push(tokio::spawn(async move {
            let mut x = 12345u32.wrapping_add(t * 977);
            for _ in 0..400_000 {
                x = x.wrapping_mul(1664525).wrapping_add(1013904223);
                let k = (x >> 16) % 16;
                if x & 1 == 0 { c.insert(k, x).await; } else { c.invalidate(&k).await; }
            }
        }));
    }
    for h in hs { h.await.unwrap(); }
    for k in 0..16 { c.invalidate(&k).await; }
    for _ in 0..5 { c.run_pending_tasks().await; }
    println!("future cache after churn: entry_count={} weighted={} actual={}", c.entry_count(), c.weighted_size(), c.iter().count());
    for k in 100..1100 { c.insert(k, k).await; }
    c.run_pending_tasks().await;
    let mut hits = 0; for k in 100..1100 { if c.contains_key(&k) { hits += 1; } }
    println!("  1000 fresh keys into capacity-1000: {hits} retained");
}
