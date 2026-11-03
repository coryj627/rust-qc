use moka::future::Cache as ACache;
use moka::notification::RemovalCause;
use moka::sync::{Cache, SegmentedCache};
use moka::Expiry;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

struct CreateOnly(Duration);
impl<K, V> Expiry<K, V> for CreateOnly {
    fn expire_after_create(&self, _: &K, _: &V, _: Instant) -> Option<Duration> { Some(self.0) }
}

fn expiry_reinsert() {
    let c: Cache<u32, u32> = Cache::builder().expire_after(CreateOnly(Duration::from_millis(200))).build();
    c.insert(1, 10);
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(300));
    c.insert(1, 20);
    println!("[expiry] blind re-insert after expiry: get = {:?} (expected Some(20))", c.get(&1));
    let c: ACache<u32, u32> = ACache::builder().expire_after(CreateOnly(Duration::from_millis(200))).build();
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(async {
        c.insert(1, 10).await;
        c.run_pending_tasks().await;
        tokio::time::sleep(Duration::from_millis(300)).await;
        c.insert(1, 20).await;
        println!("[expiry-async] blind re-insert after expiry: get = {:?}", c.get(&1).await);
    });
    // entry api
    let c: Cache<u32, u32> = Cache::builder().expire_after(CreateOnly(Duration::from_millis(200))).build();
    c.insert(1, 10);
    std::thread::sleep(Duration::from_millis(300));
    let e = c.entry(1).and_upsert_with(|o| { println!("[expiry-compute] saw old={:?}", o.map(|e| e.into_value())); 20 });
    println!("[expiry-compute] upsert returned {} is_fresh={}; get={:?}", e.value(), e.is_fresh(), c.get(&1));
}

fn replace_flag() {
    let c: Cache<u32, u32> = Cache::new(10);
    c.insert(1, 1);
    let e = c.entry(1).or_insert_with_if(|| 2, |_| true);
    println!("[replace] or_insert_with_if: value={} fresh={} old_replaced={} (doc: true when an old value was replaced)", e.value(), e.is_fresh(), e.is_old_value_replaced());
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(async {
        let c: ACache<u32, u32> = ACache::new(10);
        c.insert(1, 1).await;
        let e = c.entry(1).or_insert_with_if(async { 2 }, |_| true).await;
        println!("[replace-async] value={} fresh={} old_replaced={}", e.value(), e.is_fresh(), e.is_old_value_replaced());
    });
}

fn two_pred() {
    let c: Cache<u32, u32> = Cache::builder().support_invalidation_closures().build();
    for i in 0..1000u32 { c.insert(i, i); if i % 32 == 0 { c.run_pending_tasks(); } }
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(2));
    c.invalidate_entries_if(|_, _| false).unwrap(); // A
    std::thread::sleep(Duration::from_millis(2));
    let mut next = 1000u32;
    for round in 0..16 {
        for _ in 0..60 { c.insert(next, next); next += 1; }
        c.run_pending_tasks();
        if round == 1 { c.invalidate_entries_if(|k, _| *k < 1000).unwrap(); } // B
    }
    for _ in 0..40 { c.run_pending_tasks(); }
    let left = (0..1000u32).filter(|k| c.contains_key(k)).count();
    println!("[two-pred] readable keys that predicate B should have removed: {left} of 1000");
}

fn notifier_causes() {
    // ttl-expired entries replaced by insert should report Expired, others Replaced.
    let events = Arc::new(Mutex::new(vec![]));
    let e2 = events.clone();
    let c: Cache<u32, u32> = Cache::builder()
        .time_to_live(Duration::from_millis(100))
        .eviction_listener(move |k, v, cause| e2.lock().unwrap().push((*k, v, cause)))
        .build();
    c.insert(1, 1);
    c.insert(1, 2);
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(150));
    c.insert(1, 3);
    c.run_pending_tasks();
    c.invalidate(&1);
    c.insert(2, 1); c.invalidate_all(); c.run_pending_tasks(); c.insert(2, 2);
    c.run_pending_tasks();
    println!("[notify] {:?}", events.lock().unwrap());
    let _ = RemovalCause::Explicit;
}

fn weighted_seg() {
    // SegmentedCache capacity rounding
    let c: SegmentedCache<u32, u32> = SegmentedCache::new(8, 16);
    for i in 0..1000 { c.insert(i, i); if i % 10 == 0 { c.run_pending_tasks(); } }
    c.run_pending_tasks();
    println!("[seg] new(8,16): policy.max={:?} entry_count={}", c.policy().max_capacity(), c.entry_count());
    let c: Cache<u32, Vec<u8>> = Cache::builder().max_capacity(100).weigher(|_, v: &Vec<u8>| v.len() as u32).build();
    c.insert(1, vec![0; 50]);
    c.run_pending_tasks();
    c.insert(1, vec![0; 500]);
    c.run_pending_tasks();
    println!("[weigh] grown beyond cap: get is_some={} weighted={} count={}", c.get(&1).is_some(), c.weighted_size(), c.entry_count());
    c.insert(2, vec![0; 500]);
    println!("[weigh] oversize before maintenance visible={}", c.contains_key(&2));
    c.run_pending_tasks();
    println!("[weigh] oversize after maintenance visible={}", c.contains_key(&2));
    let c: Cache<u32, Vec<u8>> = Cache::builder().max_capacity(100).weigher(|_, v: &Vec<u8>| v.len() as u32).build();
    for i in 0..50u32 { c.insert(i, vec![0; 2]); }
    c.run_pending_tasks();
    // weight zero
    for i in 100..300u32 { c.insert(i, vec![]); }
    c.run_pending_tasks();
    println!("[weigh0] count={} weighted={}", c.entry_count(), c.weighted_size());
}

fn iter_seg() {
    let c: SegmentedCache<u32, u32> = SegmentedCache::new(100_000, 3);
    for i in 0..5000 { c.insert(i, i); }
    let mut seen: Vec<u32> = c.iter().map(|(k, _)| *k).collect();
    seen.sort();
    let n = seen.len(); seen.dedup();
    println!("[iter] seg iter n={} dedup={}", n, seen.len());
}

fn expiry_panic() {
    // Panic in expire_after_create after insert in map.
    struct P;
    impl Expiry<u32, u32> for P {
        fn expire_after_create(&self, _: &u32, v: &u32, _: Instant) -> Option<Duration> { if *v == 666 { panic!("expiry") } None }
    }
    let c: Cache<u32, u32> = Cache::builder().max_capacity(10).expire_after(P).build();
    let _ = catch_unwind(AssertUnwindSafe(|| c.insert(1, 666)));
    c.run_pending_tasks();
    println!("[expiry-panic] after panicking insert: get={:?} entry_count={} iter={}", c.get(&1), c.entry_count(), c.iter().count());
    for i in 100..1000 { c.insert(i, i); if i % 16 == 0 { c.run_pending_tasks(); } }
    c.run_pending_tasks();
    println!("[expiry-panic] after 900 more inserts (cap 10): get(1)={:?} entry_count={} iter={}", c.get(&1), c.entry_count(), c.iter().count());
}

fn listener_reentry() {
    let c: Arc<Mutex<Option<Cache<u32,u32>>>> = Arc::new(Mutex::new(None));
    let _ = c;
}

fn tti_read_only_expiry() {
    // get via contains_key/iter does not extend idle.
    let n = Arc::new(AtomicU64::new(0));
    let _ = n.fetch_add(0, Ordering::Relaxed);
    let c: Cache<u32,u32> = Cache::builder().time_to_idle(Duration::from_millis(200)).build();
    c.insert(1,1);
    for _ in 0..6 { std::thread::sleep(Duration::from_millis(60)); assert_eq!(c.get(&1), Some(1)); }
    std::thread::sleep(Duration::from_millis(250));
    println!("[tti] after idle: {:?}", c.get(&1));
}

fn main() {
    let which: Vec<String> = std::env::args().skip(1).collect();
    let all = which.is_empty();
    let has = |s: &str| all || which.iter().any(|w| w == s);
    std::panic::set_hook(Box::new(|i| eprintln!("  panic: {i}")));
    if has("expiry") { expiry_reinsert(); }
    if has("replace") { replace_flag(); }
    if has("twopred") { two_pred(); }
    if has("notify") { notifier_causes(); }
    if has("weighted") { weighted_seg(); }
    if has("iter") { iter_seg(); }
    if has("epanic") { expiry_panic(); }
    if has("tti") { tti_read_only_expiry(); }
    listener_reentry();
    std::process::exit(0);
}
