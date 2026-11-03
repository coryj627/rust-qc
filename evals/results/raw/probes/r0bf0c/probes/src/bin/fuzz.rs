// Multi-threaded random ops. Arg 1: "none" lets Expiry return None. Arg 2: seconds.
use moka::{sync::{Cache, SegmentedCache}, Expiry};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
static SEED: AtomicU64 = AtomicU64::new(88172645463325252);
fn rnd() -> u64 { let mut x = SEED.fetch_add(0x9E3779B97F4A7C15, Ordering::Relaxed); x ^= x << 13; x ^= x >> 7; x ^= x << 17; x }
struct E { none: bool }
impl E { fn d(&self) -> Option<Duration> {
    let r = rnd() % 100;
    if self.none && r < 15 { None } else if r < 30 { Some(Duration::ZERO) } else { Some(Duration::from_millis(rnd() % 3000)) }
} }
impl Expiry<u32, u32> for E {
    fn expire_after_create(&self, _: &u32, _: &u32, _: Instant) -> Option<Duration> { self.d() }
    fn expire_after_read(&self, _: &u32, _: &u32, _: Instant, c: Option<Duration>, _: Instant) -> Option<Duration> { if rnd() % 4 == 0 { self.d() } else { c } }
    fn expire_after_update(&self, _: &u32, _: &u32, _: Instant, c: Option<Duration>) -> Option<Duration> { if rnd() % 2 == 0 { self.d() } else { c } }
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let none = a[1] == "none";
    let secs: u64 = a[2].parse().unwrap();
    let deadline = Instant::now() + Duration::from_secs(secs);
    let mut round = 0u32;
    while Instant::now() < deadline {
        round += 1;
        let cap = [0u64, 1, 16, 512, 4096][(rnd() % 5) as usize];
        let c: Cache<u32, u32> = Cache::builder().max_capacity(cap).weigher(|_, v: &u32| v % 8)
            .expire_after(E { none }).support_invalidation_closures()
            .eviction_listener(|_, _, _| {}).time_to_idle(Duration::from_secs(2)).build();
        let s: SegmentedCache<u32, u32> = SegmentedCache::builder(4).max_capacity(cap).time_to_live(Duration::from_millis(500)).support_invalidation_closures().build();
        let mut hs = vec![];
        for _ in 0..6 {
            let c = c.clone(); let s = s.clone();
            hs.push(std::thread::spawn(move || {
                let until = Instant::now() + Duration::from_secs(3);
                while Instant::now() < until {
                    let k = (rnd() % 700) as u32; let v = rnd() as u32;
                    match rnd() % 22 {
                        0..=5 => { c.insert(k, v); s.insert(k, v); }
                        6..=11 => { c.get(&k); s.get(&k); }
                        12 => { c.invalidate(&k); s.invalidate(&k); }
                        13 => { c.get_with(k, || v); s.get_with(k, || v); }
                        14 => { let _ = c.entry(k).or_insert_with_if(|| v, |x| x % 2 == 0); }
                        15 => { c.entry(k).and_upsert_with(|e| e.map(|e| e.into_value().wrapping_add(1)).unwrap_or(v)); }
                        16 => { if rnd() % 300 == 0 { c.invalidate_all(); s.invalidate_all(); } }
                        17 => { if rnd() % 300 == 0 { let m = rnd() as u32 % 5; let _ = c.invalidate_entries_if(move |k, _| k % 5 == m); let _ = s.invalidate_entries_if(move |k, _| k % 5 == m); } }
                        18 => { if rnd() % 50 == 0 { c.run_pending_tasks(); s.run_pending_tasks(); } }
                        19 => { if rnd() % 300 == 0 { let _ = c.iter().count(); let _ = s.iter().count(); } }
                        20 => { c.remove(&k); }
                        _ => { let _ = c.try_get_with(k, || if v % 2 == 0 { Ok(v) } else { Err(1u8) }).ok(); }
                    }
                }
            }));
        }
        for h in hs { h.join().unwrap(); }
        c.run_pending_tasks(); s.run_pending_tasks();
    }
    println!("fuzz done rounds={round} none={none}");
}
