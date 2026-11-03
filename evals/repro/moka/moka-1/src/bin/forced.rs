//! moka-1: a stale WriteOp::Upsert admits an entry already gone from the hash table.
//! The orphan deque node sits at the LRU front and eviction stops for good.
//!
//! The writer's Expiry callback runs after the hash table update and before the op is
//! queued. Holding it there forces the race with an ordinary remove, using only public
//! API and no timing. Then fresh keys are inserted, and the entry count is measured
//! against the number inserted. Max capacity is 100.
//! Exit 0 = cache grew linearly past max_capacity while the control stayed bounded.
use moka::{policy::EvictionPolicy, sync::Cache, Expiry};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc, Mutex,
};
use std::time::{Duration, Instant};

const MAX: u64 = 100;
const K: u64 = 0;
const SIZES: [u64; 4] = [2_000, 4_000, 8_000, 16_000];

struct Hold {
    armed: Arc<AtomicBool>,
    entered: Mutex<mpsc::Sender<()>>,
    release: Mutex<mpsc::Receiver<()>>,
}
impl Expiry<u64, Vec<u8>> for Hold {
    fn expire_after_update(&self, _: &u64, _: &Vec<u8>, _: Instant, _: Option<Duration>) -> Option<Duration> {
        if self.armed.swap(false, Ordering::SeqCst) {
            self.entered.lock().unwrap().send(()).unwrap();
            self.release.lock().unwrap().recv().unwrap();
        }
        None
    }
}

fn rss_kib() -> u64 {
    let s = std::fs::read_to_string("/proc/self/status").unwrap();
    s.lines()
        .find(|l| l.starts_with("VmRSS:"))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

/// Returns entry_count after each size step.
fn run(force_race: bool) -> Vec<u64> {
    let armed = Arc::new(AtomicBool::new(false));
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let cache: Cache<u64, Vec<u8>> = Cache::builder()
        .max_capacity(MAX)
        .eviction_policy(EvictionPolicy::lru())
        .expire_after(Hold {
            armed: Arc::clone(&armed),
            entered: Mutex::new(entered_tx),
            release: Mutex::new(release_rx),
        })
        .build();

    cache.insert(K, vec![1]);
    cache.run_pending_tasks();

    if force_race {
        armed.store(true, Ordering::SeqCst);
        let c = cache.clone();
        let writer = std::thread::spawn(move || c.insert(K, vec![2]));
        entered_rx.recv().unwrap(); // K's update is in the table; its op is not queued yet.
        cache.invalidate(&K);
        cache.run_pending_tasks(); // The entry is removed and de-admitted.
        release_tx.send(()).unwrap();
        writer.join().unwrap(); // The now stale op is queued.
        cache.run_pending_tasks(); // It admits the dead entry: orphan node.
    }
    cache.insert(K, vec![3]);
    cache.run_pending_tasks();

    let mut out = vec![];
    let mut next = 1_000u64;
    for &size in &SIZES {
        while next < 1_000 + size {
            cache.insert(next, vec![0u8; 1024]);
            next += 1;
            if next % 256 == 0 {
                cache.run_pending_tasks();
            }
        }
        for _ in 0..64 {
            cache.run_pending_tasks();
        }
        let ec = cache.entry_count();
        println!(
            "  inserted {size:>6} fresh keys: entry_count={ec:>6} weighted_size={:>6} rss={} KiB",
            cache.weighted_size(),
            rss_kib()
        );
        out.push(ec);
    }
    out
}

fn main() {
    println!("control, ordinary order (max_capacity {MAX}):");
    let control = run(false);
    println!("forced stale op (max_capacity {MAX}):");
    let forced = run(true);
    let bounded = control.iter().all(|&c| c <= MAX);
    let last = *forced.last().unwrap();
    let linear = forced.iter().zip(SIZES).all(|(&c, s)| c as f64 >= 0.9 * s as f64);
    println!("control bounded: {bounded}; forced grows with inserts: {linear}; final {last}");
    std::process::exit(if bounded && linear { 0 } else { 1 });
}
