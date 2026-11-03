use moka::future::Cache as FC;
use moka::sync::Cache as SC;
use moka::ops::compute::Op;
use std::sync::Arc;
#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let c: FC<u32, u64> = FC::new(100);
    let mut hs = vec![];
    for _ in 0..8 { let c = c.clone(); hs.push(tokio::spawn(async move {
        for _ in 0..5000 { c.entry(1).and_upsert_with(|e| async move { e.map(|e| e.into_value()).unwrap_or(0) + 1 }).await; }
    })); }
    for h in hs { h.await.unwrap(); }
    println!("future and_upsert_with: {:?} expected 40000", c.get(&1).await);
    let c: FC<u32, u64> = FC::new(100);
    let mut hs = vec![];
    for _ in 0..8 { let c = c.clone(); hs.push(tokio::spawn(async move {
        for _ in 0..5000 { c.entry(1).and_compute_with(|e| async move { Op::Put(e.map(|e| e.into_value()).unwrap_or(0) + 1) }).await; }
    })); }
    for h in hs { h.await.unwrap(); }
    println!("future and_compute_with: {:?} expected 40000", c.get(&1).await);
    let s: Arc<SC<u32,u64>> = Arc::new(SC::new(100));
    let mut hs = vec![];
    for _ in 0..8 { let c = s.clone(); hs.push(std::thread::spawn(move || {
        for _ in 0..5000 { c.entry(1).and_upsert_with(|e| e.map(|e| e.into_value()).unwrap_or(0) + 1); }
    })); }
    for h in hs { h.join().unwrap(); }
    println!("sync and_upsert_with: {:?} expected 40000", s.get(&1));
}
