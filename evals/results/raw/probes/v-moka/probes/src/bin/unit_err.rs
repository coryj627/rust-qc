use moka::{sync::Cache, future::Cache as FC};
use std::time::Duration;
#[tokio::main(flavor="multi_thread", worker_threads=2)]
async fn main() {
    std::panic::set_hook(Box::new(|i| println!("panic: {i}")));
    let c: Cache<u32,u32> = Cache::new(10);
    let c1=c.clone();
    let t=std::thread::spawn(move || { let r: Result<u32,std::sync::Arc<()>> = c1.try_get_with(1, || { std::thread::sleep(Duration::from_millis(300)); Err(()) }); r.is_err() });
    std::thread::sleep(Duration::from_millis(100));
    let r=std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| c.get_with(1, || 5)));
    println!("sync get_with panicked={} try={:?}", r.is_err(), t.join());
    let c: FC<u32,u32> = FC::new(10);
    let c1=c.clone();
    let t=tokio::spawn(async move { let r: Result<u32,std::sync::Arc<()>> = c1.try_get_with(1, async { tokio::time::sleep(Duration::from_millis(300)).await; Err(()) }).await; r.is_err() });
    tokio::time::sleep(Duration::from_millis(100)).await;
    let c2=c.clone();
    let r=tokio::spawn(async move { c2.get_with(1, async { 5 }).await }).await;
    println!("future get_with panicked={} try={:?}", r.is_err(), t.await.ok());
}
