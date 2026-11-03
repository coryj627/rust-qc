use std::{time::{Duration, Instant}, sync::{Arc,Mutex}};
struct Exp;
impl moka::Expiry<u32,u32> for Exp {
    fn expire_after_create(&self, _: &u32, _: &u32, _: Instant) -> Option<Duration> { Some(Duration::from_millis(20)) }
}
fn main() {
    let messages = Arc::new(Mutex::new(Vec::new()));
    let m = messages.clone();
    let c = moka::sync::Cache::builder().expire_after(Exp).eviction_listener(move |k,v,cause| m.lock().unwrap().push((*k,v,cause))).build();
    c.insert(1,1); c.insert(2,2); c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(40));
    println!("contains {:?}, remove {:?}", c.contains_key(&1), c.remove(&1));
    c.insert(2,3);
    println!("notifications {:?}", messages.lock().unwrap());
}
