// control: sync::Cache
use moka::sync::Cache;
fn main() {
    for round in 0..3 {
        let cache: Cache<u32, u64> = Cache::new(100);
        let mut hs = vec![];
        for _ in 0..16 {
            let c = cache.clone();
            hs.push(std::thread::spawn(move || {
                for _ in 0..2000 {
                    c.entry(1).and_upsert_with(|e| e.map(|e| e.into_value() + 1).unwrap_or(1));
                }
            }));
        }
        for h in hs { h.join().unwrap(); }
        println!("sync round {round}: expected 32000 got {:?}", cache.get(&1));
    }
}
