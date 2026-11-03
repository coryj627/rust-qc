//! Same bug with no forced interleaving: plain threads, bounded by wall clock.
//! Exits 0 if the run finishes quietly. A crash or panic is reported by the caller.
use moka::{sync::Cache, Expiry};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

struct E;
impl Expiry<u64, Result<u64, ()>> for E {
    fn expire_after_create(&self, _: &u64, v: &Result<u64, ()>, _: Instant) -> Option<Duration> {
        v.is_err().then(|| Duration::from_millis(100))
    }
    fn expire_after_update(&self, _: &u64, v: &Result<u64, ()>, _: Instant, _: Option<Duration>) -> Option<Duration> {
        v.is_err().then(|| Duration::from_millis(100))
    }
}

fn main() {
    let secs: u64 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(5);
    let cache: Cache<u64, Result<u64, ()>> = Cache::builder()
        .expire_after(E)
        .time_to_idle(Duration::from_secs(240))
        .max_capacity(10_000)
        .build();
    let start = Instant::now();
    let dur = Duration::from_secs(secs);
    let panics = Arc::new(AtomicUsize::new(0));
    let mut hs = vec![];
    for tid in 0..3u64 {
        let c = cache.clone();
        let p = Arc::clone(&panics);
        hs.push(std::thread::spawn(move || {
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let mut ops = 0u64;
                while start.elapsed() < dur {
                    for key in 0..100u64 {
                        let key = key + tid * 10_000;
                        c.insert(key, Err(()));
                        c.insert(key, Ok(ops));
                        if ops % 3 == 0 {
                            c.insert(key, Err(()));
                        }
                        ops += 1;
                    }
                }
            }));
            if r.is_err() {
                p.fetch_add(1, Ordering::SeqCst);
            }
        }));
    }
    let c = cache.clone();
    let p = Arc::clone(&panics);
    hs.push(std::thread::spawn(move || {
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            while start.elapsed() < dur {
                c.run_pending_tasks();
            }
        }));
        if r.is_err() {
            p.fetch_add(1, Ordering::SeqCst);
        }
    }));
    for h in hs {
        let _ = h.join();
    }
    let n = panics.load(Ordering::SeqCst);
    println!("thread panics: {n}");
    if n > 0 {
        std::process::exit(101);
    }
}
