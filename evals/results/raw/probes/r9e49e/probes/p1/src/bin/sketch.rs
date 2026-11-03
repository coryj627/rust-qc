use moka::sync::Cache;
use std::hash::{BuildHasherDefault, Hasher};
#[derive(Default, Clone)]
struct Id(u64);
impl Hasher for Id {
    fn finish(&self) -> u64 { self.0 }
    fn write(&mut self, b: &[u8]) { for x in b { self.0 = self.0 << 8 | *x as u64; } }
    fn write_u64(&mut self, v: u64) { self.0 = v; }
}
fn main() {
    std::panic::set_hook(Box::new(|i| println!("  panic: {}", i)));
    for cap in [129u64, 130, 150, 257, 300, 513, 1025, 2049, 5000] {
        let c: Cache<u64, u64, _> = Cache::builder().max_capacity(cap).build_with_hasher(BuildHasherDefault::<Id>::default());
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            for k in 0..cap { c.insert(k, k); }
            c.run_pending_tasks();
            for k in 0..(cap * 40) { let _ = c.get(&(1_000_000 + k)); if k % 32 == 0 { c.run_pending_tasks(); } }
        }));
        println!("cap {cap}: {:?}", r.map(|_| "ok").map_err(|_| "PANIC"));
    }
}
