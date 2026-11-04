use moka::Expiry;
use std::time::{Duration, Instant};
struct Exp;
impl Expiry<u32, u32> for Exp {
    fn expire_after_create(&self, _: &u32, v: &u32, _: Instant) -> Option<Duration> {
        (*v == 10).then_some(Duration::ZERO)
    }
    fn expire_after_update(
        &self,
        _: &u32,
        _: &u32,
        _: Instant,
        _: Option<Duration>,
    ) -> Option<Duration> {
        None // Documented to clear expiration.
    }
}
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut actual = Vec::new();
    let c = moka::sync::Cache::builder().expire_after(Exp).build();
    c.insert(1, 10);
    assert_eq!(c.get(&1), None);
    c.insert(1, 20);
    actual.push(c.get(&1));
    let c = moka::sync::Cache::builder().expire_after(Exp).build();
    c.insert(1, 10);
    assert_eq!(c.get_with(1, || 20), 20);
    actual.push(c.get(&1));
    let c = moka::future::Cache::builder().expire_after(Exp).build();
    c.insert(1, 10).await;
    assert_eq!(c.get(&1).await, None);
    c.insert(1, 20).await;
    actual.push(c.get(&1).await);
    let c = moka::future::Cache::builder().expire_after(Exp).build();
    c.insert(1, 10).await;
    assert_eq!(c.get_with(1, async { 20 }).await, 20);
    actual.push(c.get(&1).await);
    println!("sync insert, sync get_with, future insert, future get_with: {actual:?}");
    assert_eq!(
        actual,
        vec![Some(20); 4],
        "all replacements should be readable"
    );
}
