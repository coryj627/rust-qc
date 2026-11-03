use moka::future::Cache;
use futures_util::FutureExt;
use std::time::Duration;
#[tokio::main(flavor="multi_thread", worker_threads=2)]
async fn main() {
    let c: Cache<u32,u32> = Cache::builder().max_capacity(1).time_to_live(Duration::from_millis(10))
        .async_eviction_listener(|_k,_v,_c| async { tokio::time::sleep(Duration::from_millis(200)).await }.boxed()).build();
    c.insert(1,1).await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    let r=tokio::time::timeout(Duration::from_millis(50), c.run_pending_tasks()).await;
    println!("rpt timed out={}", r.is_err());
    tokio::time::sleep(Duration::from_millis(500)).await;
    let c2=c.clone();
    let t=tokio::spawn(async move { c2.insert(1,2).await; });
    let r=tokio::time::timeout(Duration::from_secs(4), t).await;
    println!("same-key insert completed within 4s={}", r.is_ok());
    let c3=c.clone();
    let t=tokio::spawn(async move { c3.insert(2,2).await; });
    let r=tokio::time::timeout(Duration::from_secs(4), t).await;
    println!("other-key insert completed within 4s={}", r.is_ok());
    std::process::exit(0);
}
