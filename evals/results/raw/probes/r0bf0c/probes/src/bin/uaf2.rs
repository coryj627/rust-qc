// Only insert + get + invalidate, no entry API: Expiry clears expiration on some reads.
use moka::{sync::Cache, Expiry};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
static N: AtomicU64 = AtomicU64::new(0);
struct E;
impl Expiry<u32, u32> for E {
    fn expire_after_create(&self, _: &u32, _: &u32, _: Instant) -> Option<Duration> { Some(Duration::from_millis(1500)) }
    fn expire_after_read(&self, _: &u32, _: &u32, _: Instant, c: Option<Duration>, _: Instant) -> Option<Duration> {
        if N.fetch_add(1, Ordering::Relaxed) % 5 == 0 { None } else { c }
    }
}
fn main() {
    let cache: Cache<u32, u32> = Cache::builder().max_capacity(100_000).expire_after(E).build();
    let mut hs = vec![];
    for t in 0..4u32 {
        let c = cache.clone();
        hs.push(std::thread::spawn(move || {
            let until = Instant::now() + Duration::from_secs(70);
            let mut i = t;
            while Instant::now() < until {
                i = i.wrapping_mul(2654435761).wrapping_add(1);
                let k = i % 3000;
                match i >> 28 { 0..=4 => c.insert(k, i), 5 => c.invalidate(&k), _ => { c.get(&k); } }
            }
        }));
    }
    for h in hs { h.join().unwrap(); }
    println!("survived");
}
