use moka::policy::EvictionPolicy;
use moka::sync::Cache;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn growth() {
    let c: Cache<u64, u64> = Cache::builder().max_capacity(1000).eviction_policy(EvictionPolicy::lru()).build();
    let mut hs = vec![];
    for t in 0..4u64 {
        let c = c.clone();
        hs.push(std::thread::spawn(move || {
            let start = Instant::now();
            let mut x = 88172645463325252u64.wrapping_add(t * 977);
            while start.elapsed() < Duration::from_secs(4) {
                x ^= x << 13; x ^= x >> 7; x ^= x << 17;
                let k = x % 4;
                if x & 0x3f == 0 { c.invalidate(&k); } else { c.insert(k, x); }
            }
        }));
    }
    for h in hs { h.join().unwrap(); }
    for k in 0..4u64 { c.insert(k, k); }
    for _ in 0..5 { c.run_pending_tasks(); }
    println!("[growth] after hammer: entry_count={} real={}", c.entry_count(), c.iter().count());
    for k in 1000..41000u64 { c.insert(k, k); if k % 64 == 0 { c.run_pending_tasks(); } }
    for _ in 0..50 { c.run_pending_tasks(); }
    println!("[growth] LRU cap 1000 after 40000 fresh inserts: entry_count={} weighted={} real entries={}", c.entry_count(), c.weighted_size(), c.iter().count());
}

fn overlap() {
    let n = 8usize;
    let inflight: Arc<Vec<AtomicU64>> = Arc::new((0..n).map(|_| AtomicU64::new(0)).collect());
    let overlaps = Arc::new(AtomicU64::new(0));
    let (i2, o2) = (inflight.clone(), overlaps.clone());
    let c: Cache<u64, u64> = Cache::builder().max_capacity(100).eviction_listener(move |k, _, _| {
        let s = &i2[*k as usize];
        if s.fetch_add(1, Ordering::SeqCst) != 0 { o2.fetch_add(1, Ordering::SeqCst); }
        std::hint::spin_loop();
        s.fetch_sub(1, Ordering::SeqCst);
    }).build();
    let mut hs = vec![];
    for t in 0..4u64 {
        let c = c.clone();
        hs.push(std::thread::spawn(move || {
            let start = Instant::now();
            let mut x = 88172645463325252u64.wrapping_add(t * 977);
            while start.elapsed() < Duration::from_secs(6) {
                x ^= x << 13; x ^= x >> 7; x ^= x << 17;
                c.insert(x % n as u64, x);
            }
        }));
    }
    for h in hs { h.join().unwrap(); }
    println!("[keylock] overlapping listener calls on the same key: {}", overlaps.load(Ordering::SeqCst));
}

fn main() { growth(); overlap(); std::process::exit(0); }
