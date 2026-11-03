use moka::future::Cache as ACache;
use moka::policy::EvictionPolicy;
use moka::sync::Cache;
use std::time::{Duration, Instant};

fn hammer(c: &Cache<u64, u64>, secs: u64) {
    let mut hs = vec![];
    for t in 0..4u64 {
        let c = c.clone();
        hs.push(std::thread::spawn(move || {
            let start = Instant::now();
            let mut x = 88172645463325252u64.wrapping_add(t * 977);
            while start.elapsed() < Duration::from_secs(secs) {
                x ^= x << 13; x ^= x >> 7; x ^= x << 17;
                let k = x % 4;
                if x & 0x3f == 0 { c.invalidate(&k); } else { c.insert(k, x); }
            }
        }));
    }
    for h in hs { h.join().unwrap(); }
}

fn retained(c: &Cache<u64, u64>) -> usize {
    for k in 1000..1500u64 { c.insert(k, k); if k % 32 == 0 { c.run_pending_tasks(); } }
    // Read them a few times so TinyLFU sees popularity.
    for _ in 0..3 { for k in 1000..1500u64 { let _ = c.get(&k); } c.run_pending_tasks(); }
    for k in 1000..1500u64 { c.insert(k, k); if k % 32 == 0 { c.run_pending_tasks(); } }
    c.run_pending_tasks();
    (1000..1500u64).filter(|k| c.contains_key(k)).count()
}

fn run(name: &str, policy: EvictionPolicy, secs: u64) {
    let c: Cache<u64, u64> = Cache::builder().max_capacity(1000).eviction_policy(policy).build();
    if secs > 0 { hammer(&c, secs); }
    for k in 0..4 { c.invalidate(&k); }
    for _ in 0..5 { c.run_pending_tasks(); }
    println!("[{name}] hammer {secs}s, all 4 keys removed: entry_count={} weighted={} real={}", c.entry_count(), c.weighted_size(), c.iter().count());
    let r = retained(&c);
    println!("[{name}] then 500 fresh keys into cap 1000: retained {r}; entry_count={}", c.entry_count());
}

fn asyncv() {
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(4).enable_all().build().unwrap();
    rt.block_on(async {
        let c: ACache<u64, u64> = ACache::new(1000);
        let mut hs = vec![];
        for t in 0..4u64 {
            let c = c.clone();
            hs.push(tokio::spawn(async move {
                let start = Instant::now();
                let mut x = 88172645463325252u64.wrapping_add(t * 977);
                while start.elapsed() < Duration::from_secs(5) {
                    x ^= x << 13; x ^= x >> 7; x ^= x << 17;
                    let k = x % 4;
                    if x & 0x3f == 0 { c.invalidate(&k).await; } else { c.insert(k, x).await; }
                }
            }));
        }
        for h in hs { h.await.unwrap(); }
        for k in 0..4 { c.invalidate(&k).await; }
        for _ in 0..5 { c.run_pending_tasks().await; }
        println!("[async] after removing all: entry_count={} real={}", c.entry_count(), c.iter().count());
    });
}

fn main() {
    run("tinylfu-control", EvictionPolicy::default(), 0);
    run("lru-control", EvictionPolicy::lru(), 0);
    run("tinylfu", EvictionPolicy::default(), 6);
    run("lru", EvictionPolicy::lru(), 6);
    asyncv();
    std::process::exit(0);
}
