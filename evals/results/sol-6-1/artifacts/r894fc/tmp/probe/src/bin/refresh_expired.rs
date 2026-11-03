use std::time::{Duration, Instant};
struct Exp;
impl moka::Expiry<u32,u32> for Exp {
    fn expire_after_create(&self, _: &u32, _: &u32, _: Instant) -> Option<Duration> { println!("create"); Some(Duration::from_millis(20)) }
    fn expire_after_update(&self, _: &u32, _: &u32, _: Instant, d: Option<Duration>) -> Option<Duration> { println!("update duration={d:?}; returning None to clear expiration"); None }
}
fn main() {
    let c = moka::sync::Cache::builder().max_capacity(100).expire_after(Exp).build();
    c.insert(1, 1);
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(40));
    println!("expired {:?}",c.get(&1));
    println!("get_with {}",c.get_with(1, || 2));
    println!("new value {:?}",c.get(&1));
}
