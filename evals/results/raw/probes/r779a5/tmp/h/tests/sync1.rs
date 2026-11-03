use moka::{notification::RemovalCause, sync::Cache, Expiry};
use std::{sync::{Arc, Mutex, mpsc}, time::{Duration, Instant}};

// T1: entry_gen wrap makes entry permanently dirty.
#[test]
fn t1_gen_wrap_ttl() {
    let c: Cache<u32, u32> = Cache::builder().time_to_live(Duration::from_millis(500)).build();
    for i in 0..70_000u32 { c.insert(1, i); }
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(800));
    for _ in 0..5 { c.run_pending_tasks(); }
    println!("t1 ttl: get={:?} entry_count={}", c.get(&1), c.entry_count());
    assert_eq!(c.entry_count(), 0, "expired entry never evicted");
}

#[test]
fn t1b_control_no_wrap() {
    let c: Cache<u32, u32> = Cache::builder().time_to_live(Duration::from_millis(500)).build();
    for i in 0..60_000u32 { c.insert(1, i); }
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(800));
    for _ in 0..5 { c.run_pending_tasks(); }
    println!("t1b control: get={:?} entry_count={}", c.get(&1), c.entry_count());
    assert_eq!(c.entry_count(), 0);
}

#[test]
fn t1c_gen_wrap_capacity() {
    let c: Cache<u32, u32> = Cache::builder().max_capacity(1).eviction_policy(moka::policy::EvictionPolicy::lru()).build();
    for i in 0..70_000u32 { c.insert(1, i); }
    c.run_pending_tasks();
    for k in 2..10u32 { c.insert(k, 0); c.run_pending_tasks(); }
    for _ in 0..5 { c.run_pending_tasks(); }
    println!("t1c cap: entry_count={} weighted={} k1={:?}", c.entry_count(), c.weighted_size(), c.get(&1));
    assert!(c.entry_count() <= 1);
}

// T2: predicate scan skips every (batch+1)th entry
#[test]
fn t2_invalidate_entries_if_skips() {
    let c: Cache<u32, u32> = Cache::builder().support_invalidation_closures().build();
    for i in 0..2000u32 { c.insert(i, i); }
    c.run_pending_tasks();
    assert_eq!(c.entry_count(), 2000);
    c.invalidate_entries_if(|_, _| true).unwrap();
    for _ in 0..50 { c.run_pending_tasks(); std::thread::sleep(Duration::from_millis(2)); }
    let vis: Vec<_> = c.iter().map(|(k, _)| *k).collect();
    println!("t2: entry_count={} visible={:?}", c.entry_count(), vis);
    assert!(vis.is_empty());
}

// T3: by_ref or_try_insert_with drops key on hit
#[test]
fn t3_by_ref_or_try_insert_with_key() {
    let c: Cache<String, u32> = Cache::new(10);
    c.insert("a".to_string(), 1);
    let e = c.entry_by_ref("a").or_try_insert_with(|| Err::<u32, ()>(())).unwrap();
    println!("t3: value {}", e.value());
    assert_eq!(e.key(), "a");
}
#[test]
fn t3b_owned_or_try_insert_with_key() {
    let c: Cache<String, u32> = Cache::new(10);
    c.insert("a".to_string(), 1);
    let e = c.entry("a".to_string()).or_try_insert_with(|| Err::<u32, ()>(())).unwrap();
    assert_eq!(e.key(), "a");
}

// T4: get_with vs try_get_with::<_, ()> share waiter
#[test]
fn t4_unit_error_collides() {
    let c: Cache<u32, u32> = Cache::new(10);
    let c2 = c.clone();
    let (tx, rx) = mpsc::channel();
    let a = std::thread::spawn(move || {
        c2.try_get_with(1, || { tx.send(()).unwrap(); std::thread::sleep(Duration::from_millis(300)); Err::<u32, ()>(()) })
    });
    rx.recv().unwrap();
    let v = c.get_with(1, || 42);
    println!("t4: {v} {:?}", a.join().unwrap().is_err());
    assert_eq!(v, 42);
}

// T5: weigher panic in get_with poisons key forever
#[test]
fn t5_weigher_panic_poisons() {
    use std::sync::atomic::*;
    static N: AtomicU32 = AtomicU32::new(0);
    let c: Cache<u32, u32> = Cache::builder().max_capacity(10).weigher(|_, _| {
        if N.fetch_add(1, Ordering::SeqCst) == 0 { panic!("boom") }
        1
    }).build();
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| c.get_with(1, || 1)));
    assert!(r.is_err());
    let r2 = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| c.get_with(1, || 2)));
    println!("t5: second get_with is_err={}", r2.is_err());
    let r3 = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| c.get_with(1, || 3)));
    println!("t5: third get_with is_err={}", r3.is_err());
    assert!(r2.is_ok());
}

// T7: cause for per-entry expired
struct E(Duration);
impl Expiry<u32, u32> for E {
    fn expire_after_create(&self, _: &u32, _: &u32, _: Instant) -> Option<Duration> { Some(self.0) }
}
#[test]
fn t7_cause_per_entry() {
    let log: Arc<Mutex<Vec<(u32, RemovalCause)>>> = Default::default();
    let l2 = log.clone();
    let c: Cache<u32, u32> = Cache::builder().expire_after(E(Duration::from_millis(100)))
        .eviction_listener(move |k, _, cause| l2.lock().unwrap().push((*k, cause))).build();
    c.insert(1, 1);
    c.insert(2, 2);
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(200));
    c.insert(1, 11);
    c.invalidate(&2);
    println!("t7: {:?}", log.lock().unwrap());
    assert_eq!(log.lock().unwrap()[0].1, RemovalCause::Expired);
    assert_eq!(log.lock().unwrap()[1].1, RemovalCause::Expired);
}

// control with TTL
#[test]
fn t7b_cause_ttl() {
    let log: Arc<Mutex<Vec<(u32, RemovalCause)>>> = Default::default();
    let l2 = log.clone();
    let c: Cache<u32, u32> = Cache::builder().time_to_live(Duration::from_millis(100))
        .eviction_listener(move |k, _, cause| l2.lock().unwrap().push((*k, cause))).build();
    c.insert(1, 1);
    c.insert(2, 2);
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(200));
    c.insert(1, 11);
    c.invalidate(&2);
    println!("t7b: {:?}", log.lock().unwrap());
}

// T8: remove returns logically-absent value
#[test]
fn t8_remove_after_invalidate_all() {
    let c: Cache<u32, u32> = Cache::new(10);
    c.insert(1, 1);
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(5));
    c.invalidate_all();
    assert_eq!(c.get(&1), None);
    let r = c.remove(&1);
    println!("t8: remove={r:?}");
    assert_eq!(r, None);
}
#[test]
fn t8b_remove_after_ttl() {
    let c: Cache<u32, u32> = Cache::builder().time_to_live(Duration::from_millis(50)).build();
    c.insert(1, 1);
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(c.get(&1), None);
    let r = c.remove(&1);
    println!("t8b: remove={r:?}");
    assert_eq!(r, None);
}
#[test]
fn t8c_compute_remove_after_ttl() {
    use moka::ops::compute::{CompResult, Op};
    let c: Cache<u32, u32> = Cache::builder().time_to_live(Duration::from_millis(50)).build();
    c.insert(1, 1);
    std::thread::sleep(Duration::from_millis(100));
    let r = c.entry(1).and_compute_with(|e| { assert!(e.is_none()); Op::Remove });
    println!("t8c: {r:?}");
    assert!(matches!(r, CompResult::StillNone(_)));
}

#[test]
fn t9_second_predicate_misses_before_cursor() {
    let c: Cache<u32, u32> = Cache::builder().support_invalidation_closures().build();
    for i in 0..1000u32 { c.insert(i, i); }
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(2));
    c.invalidate_entries_if(|_, _| false).unwrap(); // P1 matches nothing
    std::thread::sleep(Duration::from_millis(2));
    c.run_pending_tasks(); // first batch of P1
    for i in 1000..1400u32 { c.insert(i, i); }
    std::thread::sleep(Duration::from_millis(2));
    c.invalidate_entries_if(|k, _| *k < 1000).unwrap(); // P2 should remove all of 0..1000
    for _ in 0..50 { c.run_pending_tasks(); std::thread::sleep(Duration::from_millis(1)); }
    let mut vis: Vec<_> = c.iter().map(|(k, _)| *k).filter(|k| *k < 1000).collect();
    vis.sort();
    println!("t9: survivors of P2: {} first={:?}", vis.len(), &vis[..vis.len().min(6)]);
    assert!(vis.is_empty());
}

#[test]
fn t9b_second_predicate_head() {
    let c: Cache<u32, u32> = Cache::builder().support_invalidation_closures().build();
    for i in 0..500u32 { c.insert(i, i); }
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(2));
    c.invalidate_entries_if(|_, _| false).unwrap();
    std::thread::sleep(Duration::from_millis(2));
    c.run_pending_tasks();
    c.invalidate_entries_if(|_, _| true).unwrap();
    for _ in 0..50 { c.run_pending_tasks(); std::thread::sleep(Duration::from_millis(1)); }
    let mut vis: Vec<_> = c.iter().map(|(k, _)| *k).collect();
    vis.sort();
    println!("t9b: survivors: {vis:?}");
    assert!(vis.is_empty());
}

#[test]
fn t9c_second_predicate_registered_mid_scan() {
    let c: Cache<u32, u32> = Cache::builder().support_invalidation_closures().build();
    for i in 0..1000u32 { c.insert(i, i); }
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(2));
    c.invalidate_entries_if(|_, _| false).unwrap(); // P1 matches nothing
    std::thread::sleep(Duration::from_millis(2));
    c.run_pending_tasks(); // first batch of P1
    c.invalidate_entries_if(|k, _| *k < 1000).unwrap(); // P2 should remove all of 0..1000
    std::thread::sleep(Duration::from_millis(2));
    for i in 1000..1400u32 { c.insert(i, i); }
    for _ in 0..50 { c.run_pending_tasks(); std::thread::sleep(Duration::from_millis(1)); }
    let mut vis: Vec<_> = c.iter().map(|(k, _)| *k).filter(|k| *k < 1000).collect();
    vis.sort();
    println!("t9c: survivors of P2: {} first={:?}", vis.len(), &vis[..vis.len().min(6)]);
    assert!(vis.is_empty());
}

#[test]
fn t9d_second_predicate_registered_mid_scan_truncated() {
    let c: Cache<u32, u32> = Cache::builder().support_invalidation_closures().build();
    for i in 0..1150u32 { c.insert(i, i); }
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(2));
    c.invalidate_entries_if(|_, _| false).unwrap(); // P1 matches nothing
    std::thread::sleep(Duration::from_millis(2));
    c.run_pending_tasks(); // first batch of P1
    c.invalidate_entries_if(|k, _| *k < 1150).unwrap(); // P2 should remove all of 0..1150
    std::thread::sleep(Duration::from_millis(2));
    for i in 2000..2600u32 { c.insert(i, i); }
    for _ in 0..50 { c.run_pending_tasks(); std::thread::sleep(Duration::from_millis(1)); }
    let mut vis: Vec<_> = c.iter().map(|(k, _)| *k).filter(|k| *k < 1150).collect();
    vis.sort();
    println!("t9d: survivors of P2: {} first={:?} get(0)={:?}", vis.len(), &vis[..vis.len().min(6)], c.get(&0));
    assert!(vis.is_empty());
}

// T10: Upsert op applied after Remove op: phantom admitted entry.
struct Gate { go: Mutex<mpsc::Sender<()>>, back: Mutex<mpsc::Receiver<()>> }
impl Expiry<u32, u32> for Gate {
    fn expire_after_create(&self, k: &u32, _: &u32, _: Instant) -> Option<Duration> {
        if *k == 1 {
            self.go.lock().unwrap().send(()).unwrap();
            self.back.lock().unwrap().recv().unwrap();
        }
        None
    }
}
#[test]
fn t10_phantom_entry() {
    let (go_tx, go_rx) = mpsc::channel();
    let (back_tx, back_rx) = mpsc::channel();
    let c: Cache<u32, u32> = Cache::builder().max_capacity(10)
        .eviction_policy(moka::policy::EvictionPolicy::lru())
        .expire_after(Gate { go: Mutex::new(go_tx), back: Mutex::new(back_rx) }).build();
    let c2 = c.clone();
    let t = std::thread::spawn(move || { c2.insert(1, 100); });
    go_rx.recv().unwrap();
    assert_eq!(c.get(&1), Some(100));
    c.invalidate(&1);
    c.run_pending_tasks();
    back_tx.send(()).unwrap();
    t.join().unwrap();
    c.run_pending_tasks();
    println!("t10: get(1)={:?} entry_count={} weighted={} iter={}", c.get(&1), c.entry_count(), c.weighted_size(), c.iter().count());
    // Does it ever go away?
    c.invalidate(&1); c.invalidate_all();
    std::thread::sleep(Duration::from_millis(2));
    for _ in 0..5 { c.run_pending_tasks(); }
    println!("t10 after invalidate_all: entry_count={} weighted={}", c.entry_count(), c.weighted_size());
    assert_eq!(c.entry_count(), 0);
}

#[test]
fn t10c_phantom_stalls_lru_eviction() {
    let (go_tx, go_rx) = mpsc::channel();
    let (back_tx, back_rx) = mpsc::channel();
    let c: Cache<u32, u32> = Cache::builder().max_capacity(10)
        .eviction_policy(moka::policy::EvictionPolicy::lru())
        .expire_after(Gate { go: Mutex::new(go_tx), back: Mutex::new(back_rx) }).build();
    let c2 = c.clone();
    let t = std::thread::spawn(move || { c2.insert(1, 100); });
    go_rx.recv().unwrap();
    c.invalidate(&1);
    c.run_pending_tasks();
    back_tx.send(()).unwrap();
    t.join().unwrap();
    c.run_pending_tasks();
    // Re-insert key 1 as a live entry (Gate fires again).
    let c2 = c.clone();
    let t = std::thread::spawn(move || { c2.insert(1, 101); });
    go_rx.recv().unwrap();
    back_tx.send(()).unwrap();
    t.join().unwrap();
    c.run_pending_tasks();
    for k in 100..5000u32 { c.insert(k, k); }
    for _ in 0..5 { c.run_pending_tasks(); }
    println!("t10c: entry_count={} weighted={} iter={} (max_capacity=10)", c.entry_count(), c.weighted_size(), c.iter().count());
    assert!(c.entry_count() <= 11);
}

// Pure stress with no callbacks: phantom via racing insert/invalidate.
#[test]
fn t10b_phantom_stress() {
    let c: Cache<u32, u32> = Cache::builder().max_capacity(1_000_000).build();
    let mut hs = vec![];
    for t in 0..4u32 {
        let c = c.clone();
        hs.push(std::thread::spawn(move || {
            for i in 0..300_000u32 {
                let k = i % 16;
                if (i + t) % 2 == 0 { c.insert(k, i); } else { c.invalidate(&k); }
            }
        }));
    }
    for h in hs { h.join().unwrap(); }
    for k in 0..16 { c.invalidate(&k); }
    for _ in 0..10 { c.run_pending_tasks(); }
    println!("t10b: entry_count={} weighted={} iter={}", c.entry_count(), c.weighted_size(), c.iter().count());
    assert_eq!(c.entry_count(), 0);
}

// T11: updating an expired-but-not-evicted entry keeps the past expiration time.
struct CreateOnly(Duration);
impl Expiry<u32, u32> for CreateOnly {
    fn expire_after_create(&self, _: &u32, _: &u32, _: Instant) -> Option<Duration> { Some(self.0) }
}
#[test]
fn t11_insert_over_expired_entry_is_invisible() {
    let c: Cache<u32, u32> = Cache::builder().expire_after(CreateOnly(Duration::from_millis(50))).build();
    c.insert(1, 1);
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(100));
    c.insert(1, 2);
    let g = c.get(&1);
    println!("t11: get after re-insert = {g:?} contains={} ", c.contains_key(&1));
    c.run_pending_tasks();
    println!("t11: after maintenance get = {:?} count={}", c.get(&1), c.entry_count());
    assert_eq!(g, Some(2));
}
#[test]
fn t11b_control_entry_evicted_first() {
    let c: Cache<u32, u32> = Cache::builder().expire_after(CreateOnly(Duration::from_millis(50))).build();
    c.insert(1, 1);
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(100));
    c.invalidate(&1);
    c.insert(1, 2);
    println!("t11b: get after remove+insert = {:?}", c.get(&1));
    assert_eq!(c.get(&1), Some(2));
}
#[test]
fn t11c_get_with_over_expired_entry() {
    let c: Cache<u32, u32> = Cache::builder().expire_after(CreateOnly(Duration::from_millis(50))).build();
    c.insert(1, 1);
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(100));
    let v = c.get_with(1, || 2);
    let g = c.get(&1);
    println!("t11c: get_with={v} then get={g:?}");
    assert_eq!(g, Some(2));
}

// T10d: phantom entries consume capacity with no callbacks at all.
#[test]
fn t10d_phantom_capacity_loss() {
    let c: Cache<u32, u32> = Cache::builder().max_capacity(1000).build();
    let mut hs = vec![];
    for t in 0..4u32 {
        let c = c.clone();
        hs.push(std::thread::spawn(move || {
            for i in 0..300_000u32 {
                let k = i % 16;
                if (i + t) % 2 == 0 { c.insert(k, i); } else { c.invalidate(&k); }
            }
        }));
    }
    for h in hs { h.join().unwrap(); }
    for k in 0..16 { c.invalidate(&k); }
    for _ in 0..10 { c.run_pending_tasks(); }
    println!("t10d after stress: entry_count={} weighted={} live={}", c.entry_count(), c.weighted_size(), c.iter().count());
    for k in 1000..1500u32 { c.insert(k, k); }
    for _ in 0..10 { c.run_pending_tasks(); }
    println!("t10d after 500 fresh inserts into empty cache of capacity 1000: entry_count={} live={}", c.entry_count(), c.iter().count());
    assert_eq!(c.iter().count(), 500);
}

// T12: weighted_size drift from racing inserts, no removals involved.
#[test]
fn t12_weighted_size_drift() {
    let c: Cache<u32, u32> = Cache::builder().weigher(|_, v: &u32| *v).build();
    let mut hs = vec![];
    for t in 0..4u32 {
        let c = c.clone();
        hs.push(std::thread::spawn(move || {
            for i in 0..200_000u32 {
                let k = i % 8;
                c.insert(k, 1 + ((i.wrapping_mul(2654435761) ^ t) % 1000));
            }
        }));
    }
    for h in hs { h.join().unwrap(); }
    for _ in 0..5 { c.run_pending_tasks(); }
    let truth: u64 = c.iter().map(|(_, v)| v as u64).sum();
    println!("t12: weighted_size={} true={} entries={}", c.weighted_size(), truth, c.entry_count());
    assert_eq!(c.weighted_size(), truth);
}

#[test]
fn t12b_first_insert_race_only() {
    // Each key inserted exactly by two racing threads, weight 1 each.
    let c: Cache<u32, u32> = Cache::builder().weigher(|_, _| 1).build();
    let mut hs = vec![];
    for _ in 0..4u32 {
        let c = c.clone();
        hs.push(std::thread::spawn(move || {
            for k in 0..200_000u32 { c.insert(k, k); }
        }));
    }
    for h in hs { h.join().unwrap(); }
    for _ in 0..5 { c.run_pending_tasks(); }
    println!("t12b: weighted_size={} entries={} iter={}", c.weighted_size(), c.entry_count(), c.iter().count());
    assert_eq!(c.weighted_size(), c.entry_count());
}

#[test]
fn t13_or_insert_with_if_replaced_flag() {
    let c: Cache<u32, u32> = Cache::new(10);
    c.insert(1, 1);
    let e = c.entry(1).or_insert_with_if(|| 2, |_| true);
    println!("t13: value={} fresh={} replaced={}", e.value(), e.is_fresh(), e.is_old_value_replaced());
    assert!(e.is_old_value_replaced());
}

#[test]
fn t14_segmented_capacity() {
    let c: moka::sync::SegmentedCache<u32, u32> = moka::sync::SegmentedCache::builder(4).max_capacity(10).eviction_policy(moka::policy::EvictionPolicy::lru()).build();
    for k in 0..5000u32 { c.insert(k, k); }
    for _ in 0..5 { c.run_pending_tasks(); }
    println!("t14: entry_count={} policy.max={:?}", c.entry_count(), c.policy().max_capacity());
    assert!(c.entry_count() <= 10);
}
