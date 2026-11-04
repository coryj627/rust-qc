use std::time::{Duration, Instant};
use moka::Expiry;
#[derive(Clone)]
struct Exp;
impl Expiry<u32,u32> for Exp {
    fn expire_after_create(&self, _: &u32, _: &u32, _: Instant) -> Option<Duration> {
        Some(Duration::from_millis(40))
    }
}
#[tokio::main(flavor="current_thread")]
async fn main() {
    let c = moka::sync::Cache::builder().expire_after(Exp).build();
    c.insert(1, 10);
    std::thread::sleep(Duration::from_millis(80));
    assert_eq!(c.get(&1), None);
    c.insert(1, 20);
    println!("sync insert after expiry: {:?}", c.get(&1));
    let c = moka::future::Cache::builder().expire_after(Exp).build();
    c.insert(1, 10).await;
    tokio::time::sleep(Duration::from_millis(80)).await;
    assert_eq!(c.get(&1).await, None);
    c.insert(1, 20).await;
    println!("future insert after expiry: {:?}", c.get(&1).await);
}
