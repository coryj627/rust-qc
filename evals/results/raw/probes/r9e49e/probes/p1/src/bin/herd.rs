use moka::future::Cache;
use std::{time::Duration, sync::Arc};
use futures_util::FutureExt as _;
#[tokio::main(flavor = "current_thread")]
async fn main() {
    std::panic::set_hook(Box::new(|i| println!("  panic: {}", i)));
    let c: Cache<u32, u32> = Cache::new(100);
    let mut hs = vec![];
    // A herd of callers on the same key, all with the same deadline, during a slow backend.
    for _ in 0..300 {
        let c = c.clone();
        hs.push(tokio::spawn(async move {
            let _ = tokio::time::timeout(Duration::from_millis(50), c.get_with(1, async { tokio::time::sleep(Duration::from_secs(5)).await; 1 })).await;
        }));
    }
    let c2 = Arc::new(c.clone());
    let w = tokio::spawn(async move {
        // A patient caller without a deadline whose own init would succeed at once.
        std::panic::AssertUnwindSafe(c2.get_with(1, async { 42 })).catch_unwind().await.map_err(|_| "PANIC")
    });
    for h in hs { let _ = h.await; }
    println!("patient caller result: {:?}", w.await.unwrap());
}
