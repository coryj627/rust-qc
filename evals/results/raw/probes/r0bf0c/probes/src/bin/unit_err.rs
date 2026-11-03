// try_get_with::<()> and get_with share the waiter key.
use moka::sync::Cache;
use std::time::Duration;
fn main() {
    let cache: Cache<u32, u32> = Cache::new(10);
    let c = cache.clone();
    let t1 = std::thread::spawn(move || {
        c.try_get_with(1, || { std::thread::sleep(Duration::from_millis(500)); Err::<u32, ()>(()) }).map_err(|_| "err")
    });
    std::thread::sleep(Duration::from_millis(100));
    let c = cache.clone();
    let t2 = std::thread::spawn(move || c.get_with(1, || 42));
    println!("try_get_with: {:?}", t1.join());
    println!("get_with: {:?}", t2.join().map_err(|e| e.downcast_ref::<String>().cloned().or(e.downcast_ref::<&str>().map(|s| s.to_string()))));
}
