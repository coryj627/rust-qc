use moka::future::Cache as ACache;
use moka::notification::RemovalCause;
use moka::ops::compute::Op;
use moka::sync::{Cache, SegmentedCache};
use moka::Expiry;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Barrier, Mutex};
use std::time::{Duration, Instant};

// Same key first-inserted concurrently: out of order Upsert ops.
fn weight_inflate() {
    let c: Cache<u64, u64> = Cache::builder().max_capacity(1 << 40).build();
    let mut hs = vec![];
    let b = Arc::new(Barrier::new(4));
    for t in 0..4u64 {
        let c = c.clone(); let b = b.clone();
        hs.push(std::thread::spawn(move || {
            b.wait();
            for k in 0..400_000u64 { c.insert(k, t); }
        }));
    }
    for h in hs { h.join().unwrap(); }
    for _ in 0..5 { c.run_pending_tasks(); }
    println!("[inflate] no removes, weight 1 each: entry_count={} weighted_size={} real={}", c.entry_count(), c.weighted_size(), c.iter().count());
}

// Expiry panics in expire_after_read while in the get path.
// listener gets notified at most once per removal: count events for values.
fn notification_counts() {
    let removed = Arc::new(AtomicU64::new(0));
    let r2 = removed.clone();
    let c: Cache<u64, u64> = Cache::builder().max_capacity(100_000)
        .eviction_listener(move |_, _, _| { r2.fetch_add(1, Ordering::Relaxed); }).build();
    let inserted = Arc::new(AtomicU64::new(0));
    let mut hs = vec![];
    for t in 0..4u64 {
        let c = c.clone(); let ins = inserted.clone();
        hs.push(std::thread::spawn(move || {
            let mut x = 88172645463325252u64.wrapping_add(t * 977);
            for _ in 0..400_000 {
                x ^= x << 13; x ^= x >> 7; x ^= x << 17;
                let k = x % 64;
                if x & 7 == 0 { c.invalidate(&k); } else { c.insert(k, x); ins.fetch_add(1, Ordering::Relaxed); }
            }
        }));
    }
    for h in hs { h.join().unwrap(); }
    for k in 0..64 { c.invalidate(&k); }
    for _ in 0..5 { c.run_pending_tasks(); }
    println!("[notify-count] inserts={} removal notifications={} entry_count={} real={}", inserted.load(Ordering::Relaxed), removed.load(Ordering::Relaxed), c.entry_count(), c.iter().count());
}

// A get_with cancelled in async; waiters
fn async_cancel_waiter() {
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
    rt.block_on(async {
        let c: ACache<u32, u32> = ACache::new(100);
        // Cancel leader in the init future repeatedly
        for _ in 0..100 {
            let r = tokio::time::timeout(Duration::from_millis(1), c.get_with(1, async { tokio::time::sleep(Duration::from_secs(1)).await; 1 })).await;
            assert!(r.is_err());
        }
        println!("[cancel] after cancels {:?}", c.get_with(1, async { 2 }).await);

        // cancel in and_upsert_with
        let r = tokio::time::timeout(Duration::from_millis(1), c.entry(5).and_upsert_with(|_| async { tokio::time::sleep(Duration::from_secs(1)).await; 1 })).await;
        assert!(r.is_err());
        println!("[cancel] upsert after cancel: {:?}", tokio::time::timeout(Duration::from_secs(1), c.entry(5).and_upsert_with(|_| async { 9 })).await.map(|e| e.into_value()));
        // async unit error
        let c2 = c.clone();
        let h = tokio::spawn(async move { let r: Result<u32, Arc<()>> = c2.try_get_with(7, async { tokio::time::sleep(Duration::from_millis(200)).await; Err(()) }).await; r.is_err() });
        tokio::time::sleep(Duration::from_millis(50)).await;
        let c3 = c.clone();
        let r = tokio::spawn(async move { c3.get_with(7, async { 3 }).await }).await;
        println!("[async-unit] get_with vs try_get_with::<()>: panicked={}", r.is_err());
        let _ = h.await;
        // Op::Remove on expired etc for async
    });
}

// or_insert_with_if with replace_if panicking; and get_with panic in V::clone
#[derive(Debug)]
struct PanicClone(Arc<AtomicBool>);
impl Clone for PanicClone { fn clone(&self) -> Self { if self.0.load(Ordering::SeqCst) { panic!("clone") } PanicClone(self.0.clone()) } }

fn clone_panic() {
    let flag = Arc::new(AtomicBool::new(false));
    let c: Cache<u32, PanicClone> = Cache::new(10);
    let f = flag.clone();
    let r = catch_unwind(AssertUnwindSafe(|| { let f2 = f.clone(); f.store(true, Ordering::SeqCst); c.get_with(1, move || PanicClone(f2)) }));
    println!("[clone-panic] get_with panicked={}", r.is_err());
    flag.store(false, Ordering::SeqCst);
    let f = flag.clone();
    let r = catch_unwind(AssertUnwindSafe(|| c.get_with(1, move || PanicClone(f)).0.load(Ordering::SeqCst)));
    println!("[clone-panic] second get_with ok={} {:?}", r.is_ok(), r.err().and_then(|e| e.downcast_ref::<String>().cloned()));
}

// TTL + get_with: value lost? key-lock leak test
fn keylock_stress() {
    let c: Cache<u64, u64> = Cache::builder().max_capacity(16).eviction_listener(|_, _, _: RemovalCause| {}).build();
    let mut hs = vec![];
    for t in 0..4u64 {
        let c = c.clone();
        hs.push(std::thread::spawn(move || {
            let mut x = 88172645463325252u64.wrapping_add(t * 977);
            for _ in 0..200_000 {
                x ^= x << 13; x ^= x >> 7; x ^= x << 17;
                c.insert(x % 64, x);
            }
        }));
    }
    for h in hs { h.join().unwrap(); }
    c.run_pending_tasks();
    println!("[stress-cap] cap 16: entry_count={} real={}", c.entry_count(), c.iter().count());
}

// Expiry that lengthens lifetime: timers
struct Sliding(Duration);
impl Expiry<u32,u32> for Sliding {
    fn expire_after_create(&self, _: &u32, _: &u32, _: Instant) -> Option<Duration> { Some(self.0) }
    fn expire_after_read(&self, _: &u32, _: &u32, _: Instant, _: Option<Duration>, _: Instant) -> Option<Duration> { Some(self.0) }
    fn expire_after_update(&self, _: &u32, _: &u32, _: Instant, _: Option<Duration>) -> Option<Duration> { Some(self.0) }
}
fn timers() {
    let c: Cache<u32,u32> = Cache::builder().expire_after(Sliding(Duration::from_millis(300))).build();
    for k in 0..2000u32 { c.insert(k, k); }
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(400));
    c.run_pending_tasks();
    println!("[timer] after expiry entry_count={} (expected 0)", c.entry_count());
    let c: Cache<u32,u32> = Cache::builder().expire_after(Sliding(Duration::from_secs(70))).build();
    c.insert(1,1); c.run_pending_tasks();
    println!("[timer] long {:?}", c.get(&1));
}

fn main() {
    let which: Vec<String> = std::env::args().skip(1).collect();
    let all = which.is_empty();
    let has = |s: &str| all || which.iter().any(|w| w == s);
    std::panic::set_hook(Box::new(|i| eprintln!("  panic: {}", i.to_string().replace('\n', " | "))));
    let _ = (Op::<u8>::Nop, Mutex::new(0), SegmentedCache::<u8,u8>::new(1,1));
    if has("clone") { clone_panic(); }
    if has("cancel") { async_cancel_waiter(); }
    if has("timers") { timers(); }
    if has("inflate") { weight_inflate(); }
    if has("notify") { notification_counts(); }
    if has("cap") { keylock_stress(); }
    std::process::exit(0);
}
