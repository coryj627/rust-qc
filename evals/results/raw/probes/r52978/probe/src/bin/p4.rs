use moka::future::Cache as ACache;
use moka::ops::compute::{CompResult, Op};
use moka::sync::{Cache, SegmentedCache};
use moka::Expiry;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

fn remove_stale() {
    let c: Cache<u32, u32> = Cache::builder().time_to_live(Duration::from_millis(100)).build();
    c.insert(1, 1);
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(150));
    println!("[remove] get after ttl = {:?}; remove after ttl = {:?}", c.get(&1), c.remove(&1));
    let c: Cache<u32, u32> = Cache::new(10);
    c.insert(2, 2);
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(2));
    c.invalidate_all();
    println!("[remove] get after invalidate_all = {:?}; remove = {:?}", c.get(&2), c.remove(&2));
    let c: Cache<u32, u32> = Cache::builder().support_invalidation_closures().build();
    c.insert(3, 3);
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(2));
    c.invalidate_entries_if(|_, _| true).unwrap();
    println!("[remove] get after predicate = {:?}; remove = {:?}", c.get(&3), c.remove(&3));
    let c: Cache<u32, u32> = Cache::builder().time_to_live(Duration::from_millis(100)).build();
    c.insert(4, 4);
    std::thread::sleep(Duration::from_millis(150));
    let r = c.entry(4).and_compute_with(|e| { println!("[compute-remove] closure saw {:?}", e.map(|e| e.into_value())); Op::Remove });
    match r { CompResult::Removed(e) => println!("[compute-remove] Removed({})", e.value()), CompResult::StillNone(_) => println!("[compute-remove] StillNone"), _ => println!("other") }
}

fn recursion() {
    let c: Cache<u32, u32> = Cache::new(10);
    let c2 = c.clone();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let v = c2.get_with(1, || c2.get_with(1, || 5));
        tx.send(v).unwrap();
    });
    println!("[recursion] nested get_with same key: {:?}", rx.recv_timeout(Duration::from_secs(3)).map_err(|_| "HUNG 3s"));
}

fn weight_drift() {
    let c: Cache<u32, Vec<u8>> = Cache::builder().max_capacity(1 << 40)
        .weigher(|_, v: &Vec<u8>| v.len() as u32).build();
    let mut hs = vec![];
    for t in 0..4u32 {
        let c = c.clone();
        hs.push(std::thread::spawn(move || {
            let start = Instant::now();
            let mut x = 88172645463325252u64.wrapping_add(t as u64 * 977);
            while start.elapsed() < Duration::from_secs(6) {
                x ^= x << 13; x ^= x >> 7; x ^= x << 17;
                let k = (x % 2) as u32;
                let w = (x >> 8) % 512;
                c.insert(k, vec![0u8; w as usize]);
                if x & 0xff == 0 { c.invalidate(&k); }
            }
        }));
    }
    for h in hs { h.join().unwrap(); }
    for _ in 0..5 { c.run_pending_tasks(); }
    let actual: u64 = c.iter().map(|(_, v)| v.len() as u64).sum();
    println!("[weight] weighted_size={} actual={} entry_count={} iter={}", c.weighted_size(), actual, c.entry_count(), c.iter().count());
    for k in 0..2 { c.invalidate(&k); }
    for _ in 0..5 { c.run_pending_tasks(); }
    println!("[weight] after removing all: weighted_size={} entry_count={}", c.weighted_size(), c.entry_count());
}

fn two_pred_control() {
    let c: Cache<u32, u32> = Cache::builder().support_invalidation_closures().build();
    for i in 0..1000u32 { c.insert(i, i); if i % 32 == 0 { c.run_pending_tasks(); } }
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(2));
    let mut next = 1000u32;
    for round in 0..16 {
        for _ in 0..60 { c.insert(next, next); next += 1; }
        c.run_pending_tasks();
        if round == 1 { c.invalidate_entries_if(|k, _| *k < 1000).unwrap(); }
    }
    for _ in 0..40 { c.run_pending_tasks(); }
    let left = (0..1000u32).filter(|k| c.contains_key(k)).count();
    println!("[one-pred-control] readable keys predicate should have removed: {left} of 1000");
}

fn expiry_panic() {
    struct P;
    impl Expiry<u32, u32> for P {
        fn expire_after_create(&self, _: &u32, v: &u32, _: Instant) -> Option<Duration> { if *v == 666 { panic!("expiry") } None }
    }
    let c: Cache<u32, u32> = Cache::builder().max_capacity(10).expire_after(P).build();
    let _ = catch_unwind(AssertUnwindSafe(|| c.insert(1, 666)));
    for i in 100..600 { c.insert(i, i); if i % 16 == 0 { c.run_pending_tasks(); } }
    c.run_pending_tasks();
    println!("[expiry-panic] cap 10, after 500 inserts: get(1)={:?} entry_count={} iter={}", c.get(&1), c.entry_count(), c.iter().count());
}

fn big_durations() {
    struct E;
    impl Expiry<u32, u32> for E {
        fn expire_after_create(&self, _: &u32, _: &u32, _: Instant) -> Option<Duration> { Some(Duration::MAX) }
        fn expire_after_read(&self, _: &u32, _: &u32, _: Instant, d: Option<Duration>, _: Instant) -> Option<Duration> { d }
        fn expire_after_update(&self, _: &u32, _: &u32, _: Instant, _: Option<Duration>) -> Option<Duration> { Some(Duration::MAX) }
    }
    let c: Cache<u32, u32> = Cache::builder().expire_after(E).build();
    c.insert(1, 1); c.insert(1, 2); println!("[big] {:?}", c.get(&1)); c.run_pending_tasks();println!("[big] {:?}", c.get(&1));
    let c: Cache<u32, u32> = Cache::builder().time_to_live(Duration::from_secs(1000 * 365 * 24 * 3600)).time_to_idle(Duration::from_secs(1000 * 365 * 24 * 3600)).build();
    c.insert(1, 1); c.run_pending_tasks(); println!("[big-ttl] {:?}", c.get(&1));
    // zero caps
    let c: Cache<u32, u32> = Cache::new(0);
    println!("[zero] get_with {} ; or_insert {} ; upsert {} ; get {:?}", c.get_with(1, || 1), c.entry(1).or_insert(2).value(), c.entry(1).and_upsert_with(|_| 3).value(), c.get(&1));
    c.run_pending_tasks(); c.invalidate_all(); c.invalidate(&1); println!("[zero] iter {}", c.iter().count());
    let c: SegmentedCache<u32, u32> = SegmentedCache::new(0, 1);
    c.insert(1, 1); println!("[zero-seg] {:?} {:?}", c.get(&1), c.policy().num_segments());
    let c: SegmentedCache<u32, u32> = SegmentedCache::new(10, 1);
    c.insert(1, 1); println!("[one-seg] {:?} {}", c.get(&1), c.iter().count());
    let c: Cache<String, u32> = Cache::builder().initial_capacity(1).build();
    c.insert("é".to_string(), 1); println!("[init] {:?}", c.get("é"));
    let c: SegmentedCache<String, u32> = moka::sync::SegmentedCache::builder(7).initial_capacity(3).max_capacity(100).build();
    c.insert("é".to_string(), 1); println!("[init-seg] {:?} segs={}", c.get("é"), c.policy().num_segments());
}

fn async_stall() {
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(4).enable_all().build().unwrap();
    rt.block_on(async {
        let done = Arc::new(AtomicU64::new(0));
        let c: ACache<u64, u64> = ACache::new(1000);
        let mut hs = vec![];
        let tasks = 64; 
        for t in 0..tasks {
            let c = c.clone(); let done = done.clone();
            hs.push(tokio::spawn(async move {
                for i in 0..60_000u64 { c.insert(i.wrapping_mul(t + 7) % 5000, i).await; }
                done.fetch_add(1, Ordering::SeqCst);
            }));
        }
        let start = Instant::now();
        let mut last = 0; let mut idle = 0;
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let d = done.load(Ordering::SeqCst);
            if d == tasks { println!("[async-stall] all tasks finished in {:?}", start.elapsed()); break; }
            if d == last { idle += 1; } else { idle = 0; last = d; }
            if start.elapsed() > Duration::from_secs(90) { println!("[async-stall] done {d} of {tasks} after 90s idle={idle}"); break; }
        }
    });
}

fn nobody_else() {
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
    rt.block_on(async {
        let c: ACache<u32, u32> = ACache::new(10);
        let c2 = c.clone();
        let h = tokio::spawn(async move { c2.entry(1).and_compute_with(|_| async { tokio::time::sleep(Duration::from_millis(200)).await; Op::Put(1) }).await; });
        tokio::time::sleep(Duration::from_millis(50)).await;
        let r: Result<CompResult<u32,u32>, ()> = c.entry(1).and_try_compute_if_nobody_else(|_| async { Ok::<_, ()>(Op::Put(2)) }).await;
        println!("[nobody] while busy -> {:?}", r);
        h.await.unwrap();
        let r: Result<CompResult<u32,u32>, ()> = c.entry(1).and_try_compute_if_nobody_else(|_| async { Ok::<_, ()>(Op::Put(2)) }).await;
        println!("[nobody] idle -> {:?}; get={:?}", r, c.get(&1).await);
    });
}

fn main() {
    let which: Vec<String> = std::env::args().skip(1).collect();
    let all = which.is_empty();
    let has = |s: &str| all || which.iter().any(|w| w == s);
    std::panic::set_hook(Box::new(|i| eprintln!("  panic: {i}")));
    if has("remove") { remove_stale(); }
    if has("recursion") { recursion(); }
    if has("control") { two_pred_control(); }
    if has("epanic") { expiry_panic(); }
    if has("big") { big_durations(); }
    if has("nobody") { nobody_else(); }
    if has("weight") { weight_drift(); }
    if has("stall") { async_stall(); }
    std::process::exit(0);
}
