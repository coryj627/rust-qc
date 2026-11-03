use moka::future::Cache as ACache;
use moka::ops::compute::Op;
use moka::sync::Cache;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

fn by_ref_key() {
    let c: Cache<String, u32> = Cache::new(10);
    let k = "a".to_string();
    let e = c.entry_by_ref(&k).or_try_insert_with(|| Ok::<u32, ()>(1)).unwrap();
    println!("[byref] fresh key={:?}", e.key());
    let e = c.entry_by_ref(&k).or_try_insert_with(|| Ok::<u32, ()>(2)).unwrap();
    let r = catch_unwind(AssertUnwindSafe(|| e.key().clone()));
    println!("[byref] existing entry key() -> {:?}", r.map_err(|e| e.downcast_ref::<String>().cloned().or(e.downcast_ref::<&str>().map(|s| s.to_string()))));
}

fn stress_phantom() {
    // Upsert/Remove reordering leaves entries in the policy deques.
    let c: Cache<u32, u32> = Cache::builder().max_capacity(100_000).build();
    let mut hs = vec![];
    for t in 0..4u32 {
        let c = c.clone();
        hs.push(std::thread::spawn(move || {
            let start = std::time::Instant::now();
            let mut i = t;
            while start.elapsed() < Duration::from_secs(8) {
                let k = i % 32;
                if i % 2 == 0 { c.insert(k, i); } else { c.invalidate(&k); }
                i = i.wrapping_mul(2654435761).wrapping_add(1);
            }
        }));
    }
    for h in hs { h.join().unwrap(); }
    for k in 0..32 { c.invalidate(&k); }
    for _ in 0..5 { c.run_pending_tasks(); }
    let live = c.iter().count();
    println!("[phantom] after invalidating every key: entry_count={} weighted={} iter={}", c.entry_count(), c.weighted_size(), live);
}

fn counter_async() {
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(4).enable_all().build().unwrap();
    rt.block_on(async {
        for (name, mode) in [("and_upsert_with", 0), ("and_compute_with", 1)] {
            let c: ACache<u32, u64> = ACache::new(100);
            let tasks = 8; let n = 20_000u64;
            let mut hs = vec![];
            for _ in 0..tasks {
                let c = c.clone();
                hs.push(tokio::spawn(async move {
                    for _ in 0..n {
                        if mode == 0 {
                            c.entry(1).and_upsert_with(|e| { let v = e.map(|e| e.into_value()).unwrap_or(0) + 1; std::future::ready(v) }).await;
                        } else {
                            c.entry(1).and_compute_with(|e| { let v = e.map(|e| e.into_value()).unwrap_or(0) + 1; std::future::ready(Op::Put(v)) }).await;
                        }
                    }
                }));
            }
            for h in hs { h.await.unwrap(); }
            println!("[async-counter] {name}: got {:?} expected {}", c.get(&1).await, tasks as u64 * n);
        }
    });
}
fn counter_sync() {
    let c: Cache<u32, u64> = Cache::new(100);
    let mut hs = vec![];
    for _ in 0..8 {
        let c = c.clone();
        hs.push(std::thread::spawn(move || {
            for _ in 0..20_000 { c.entry(1).and_upsert_with(|e| e.map(|e| e.into_value()).unwrap_or(0) + 1); }
        }));
    }
    for h in hs { h.join().unwrap(); }
    println!("[sync-counter] got {:?} expected {}", c.get(&1), 160000);
}

fn async_panics() {
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
    rt.block_on(async {
        // A predicate which panics once.
        let hits = Arc::new(AtomicU64::new(0));
        let c: ACache<u32, u32> = ACache::builder().support_invalidation_closures().build();
        c.insert(1, 1).await; c.insert(2, 2).await;
        c.run_pending_tasks().await;
        let h = hits.clone();
        c.invalidate_entries_if(move |_, _| { if h.fetch_add(1, Ordering::SeqCst) == 0 { panic!("once") } false }).unwrap();
        let c2 = c.clone();
        let r = tokio::spawn(async move { c2.run_pending_tasks().await }).await;
        println!("[async-pred] first run_pending_tasks ok={}", r.is_ok());
        for i in 0..3 {
            let c2 = c.clone();
            let r = tokio::spawn(async move { c2.run_pending_tasks().await }).await;
            println!("[async-pred] later run_pending_tasks #{i} ok={} err={:?}", r.is_ok(), r.err().map(|e| { let p = e.into_panic(); p.downcast_ref::<String>().cloned().or(p.downcast_ref::<&str>().map(|s| s.to_string())) }));
        }
        let c2 = c.clone();
        let r = tokio::spawn(async move { for i in 0..2000u32 { c2.insert(i, i).await; } }).await;
        println!("[async-pred] inserts afterwards ok={}", r.is_ok());
    });
}

fn multi_predicate_cursor() {
    let c: Cache<u32, u32> = Cache::builder().support_invalidation_closures().build();
    // Old entries.
    for i in 0..1000u32 { c.insert(i, i); }
    c.run_pending_tasks();
    // predicate A matches nothing.
    c.invalidate_entries_if(|_, _| false).unwrap();
    std::thread::sleep(Duration::from_millis(2));
    // Newer entries than A.
    for i in 1000..1400u32 { c.insert(i, i); if i % 32 == 0 {c.run_pending_tasks();} }
    c.run_pending_tasks();
    for _ in 0..1 { c.run_pending_tasks(); }
    // B matches everything.
    c.invalidate_entries_if(|_, _| true).unwrap();
    for _ in 0..40 { c.run_pending_tasks(); }
    let left = (0..1400u32).filter(|k| c.contains_key(k)).count();
    println!("[two-pred] after both predicates done, readable keys matching B = {left} (expected 0)");
}

fn main() {
    let which: Vec<String> = std::env::args().skip(1).collect();
    let all = which.is_empty();
    let has = |s: &str| all || which.iter().any(|w| w == s);
    std::panic::set_hook(Box::new(|i| eprintln!("  panic: {i}")));
    if has("byref") { by_ref_key(); }
    if has("counter") { counter_sync(); counter_async(); }
    if has("phantom") { stress_phantom(); }
    if has("apanic") { async_panics(); }
    if has("twopred") { multi_predicate_cursor(); }
    std::process::exit(0);
}
