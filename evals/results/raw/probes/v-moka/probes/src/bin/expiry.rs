use moka::{sync::Cache, future::Cache as FC, Expiry};
use std::time::{Duration, Instant};
struct E;
impl Expiry<u32, u32> for E {
    fn expire_after_create(&self, _: &u32, _: &u32, _: Instant) -> Option<Duration> { Some(Duration::from_millis(100)) }
}
#[tokio::main]
async fn main() {
    let c: Cache<u32,u32> = Cache::builder().max_capacity(100).expire_after(E).build();
    c.insert(1, 1);
    std::thread::sleep(Duration::from_millis(200));
    c.insert(1, 2);
    println!("sync reinsert over expired: get={:?}", c.get(&1));
    let v = c.get_with(1, || 3);
    println!("get_with -> {v}; then get={:?}", c.get(&1));
    c.run_pending_tasks();
    println!("after rpt get={:?} contains={}", c.get(&1), c.contains_key(&1));
    // control: not-expired
    let c: Cache<u32,u32> = Cache::builder().max_capacity(100).expire_after(E).build();
    c.insert(1, 1); c.insert(1, 2);
    println!("control sync live reinsert get={:?}", c.get(&1));
    let c: FC<u32,u32> = FC::builder().max_capacity(100).expire_after(E).build();
    c.insert(1, 1).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    c.insert(1, 2).await;
    println!("future reinsert over expired: get={:?}", c.get(&1).await);
}
