//! moka-1 with no forced interleaving: plain writer threads, fixed op counts. Reports how
//! many rounds end stalled above max_capacity after housekeeping has drained. The weigher
//! charges 64 KiB per entry while values stay small, which keeps real memory low.
use moka::{policy::EvictionPolicy, sync::Cache};

const MAX: u64 = 10 * 1024 * 1024;
const W: u32 = 64 * 1024;

fn main() {
    let rounds: u32 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(6);
    let mut stalled = 0;
    for r in 0..rounds {
        let cache: Cache<u64, u8> = Cache::builder()
            .max_capacity(MAX)
            .weigher(|_, _| W)
            .eviction_policy(EvictionPolicy::lru())
            .build();
        let mut hs = vec![];
        for t in 0..16u64 {
            let c = cache.clone();
            hs.push(std::thread::spawn(move || {
                for i in 0..4_000u64 {
                    if t % 2 == 0 {
                        c.insert(i % 500, 0);
                    } else {
                        c.insert((t + 1) * 1_000_000_000 + i, 0);
                    }
                }
            }));
        }
        for h in hs {
            h.join().unwrap();
        }
        for _ in 0..64 {
            cache.run_pending_tasks();
        }
        let ws = cache.weighted_size();
        let is_stalled = ws > MAX;
        stalled += is_stalled as u32;
        println!("round {r}: entry_count={} (limit {}) stalled={is_stalled}", cache.entry_count(), MAX / W as u64);
    }
    println!("plain stress: {stalled} of {rounds} rounds stalled");
}
