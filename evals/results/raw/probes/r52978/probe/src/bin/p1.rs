use moka::sync::Cache;
use std::collections::HashSet;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::time::Duration;

// 1. invalidate_entries_if misses entries past a batch.
fn invalidate_scan() {
    let n = 2000u32;
    let c: Cache<u32, u32> = Cache::builder().support_invalidation_closures().build();
    for i in 0..n { c.insert(i, i); }
    c.run_pending_tasks();
    println!("[inv] entry_count before = {}", c.entry_count());
    c.invalidate_entries_if(|_, _| true).unwrap();
    println!("[inv] get(0) immediately after registering = {:?}", c.get(&0));
    for _ in 0..40 { c.run_pending_tasks(); }
    let mut left: Vec<u32> = (0..n).filter(|k| c.contains_key(k)).collect();
    left.sort();
    println!("[inv] after scan: entry_count={} still readable={} keys={:?}", c.entry_count(), left.len(), &left[..left.len().min(20)]);
}

// 2. policy_gen wrap-around.
fn gen_wrap() {
    let c: Cache<u32, u32> = Cache::builder()
        .time_to_live(Duration::from_millis(500))
        .build();
    for i in 0..70_000u32 { c.insert(1, i); if i % 64 == 0 { c.run_pending_tasks(); } }
    c.run_pending_tasks();
    println!("[gen] after 70000 updates entry_count={}", c.entry_count());
    std::thread::sleep(Duration::from_millis(700));
    for _ in 0..5 { c.run_pending_tasks(); }
    println!("[gen] ttl passed: get={:?} entry_count={} (expected 0)", c.get(&1), c.entry_count());

    let c: Cache<u32, u32> = Cache::builder()
        .time_to_live(Duration::from_millis(500))
        .build();
    for i in 0..60_000u32 { c.insert(1, i); if i % 64 == 0 { c.run_pending_tasks(); } }
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(700));
    for _ in 0..5 { c.run_pending_tasks(); }
    println!("[gen-control] 60000 updates, ttl passed: entry_count={} (expected 0)", c.entry_count());

    // size eviction
    let c: Cache<u32, u32> = Cache::builder().max_capacity(100).eviction_policy(moka::policy::EvictionPolicy::lru()).build();
    for i in 0..70_000u32 { c.insert(1, i); if i % 64 == 0 { c.run_pending_tasks(); } }
    c.run_pending_tasks();
    for k in 2..2000u32 { c.insert(k, k); if k % 32 == 0 { c.run_pending_tasks(); } }
    for _ in 0..5 { c.run_pending_tasks(); }
    println!("[gen-lru] key 1 retained after 2000 newer keys under LRU cap 100: {:?} entry_count={}", c.get(&1), c.entry_count());

    // invalidate_entries_if
    let c: Cache<u32, u32> = Cache::builder().support_invalidation_closures().build();
    for i in 0..70_000u32 { c.insert(1, i); if i % 64 == 0 { c.run_pending_tasks(); } }
    c.run_pending_tasks();
    c.invalidate_entries_if(|_, _| true).unwrap();
    for _ in 0..5 { c.run_pending_tasks(); }
    println!("[gen-inv] after predicate scan get(1)={:?}", c.get(&1));
}

// 3. panics in user closures while the waiter is installed.
fn waiter_poison() {
    let c: Cache<u32, String> = Cache::builder()
        .weigher(|_k, v: &String| { if v == "bad" { panic!("weigher panic") } 1 })
        .max_capacity(1000)
        .build();
    let r = catch_unwind(AssertUnwindSafe(|| c.get_with(1, || "bad".to_string())));
    println!("[poison] first get_with panicked={}", r.is_err());
    let r = catch_unwind(AssertUnwindSafe(|| c.get_with(1, || "good".to_string())));
    match r {
        Ok(v) => println!("[poison] second get_with -> {v}"),
        Err(e) => println!("[poison] second get_with PANICKED: {:?}", e.downcast_ref::<String>()),
    }
    let r = catch_unwind(AssertUnwindSafe(|| c.get_with(1, || "good".to_string())));
    println!("[poison] third get_with panicked={}", r.is_err());
    c.insert(2, "x".into());
}

fn compute_hang() {
    let c: Cache<u32, String> = Cache::builder()
        .weigher(|_k, v: &String| { if v == "bad" { panic!("weigher panic") } 1 })
        .max_capacity(1000)
        .build();
    let r = catch_unwind(AssertUnwindSafe(|| c.entry(1).and_upsert_with(|_| "bad".to_string())));
    println!("[compute] first and_upsert_with panicked={}", r.is_err());
    let c2 = c.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let e = c2.entry(1).and_upsert_with(|_| "good".to_string());
        tx.send(e.into_value()).unwrap();
    });
    match rx.recv_timeout(Duration::from_secs(5)) {
        Ok(v) => println!("[compute] second and_upsert_with -> {v}"),
        Err(_) => println!("[compute] second and_upsert_with HUNG for 5s"),
    }
}

// 4. get_with vs try_get_with::<()>.
fn unit_err() {
    let c: Cache<u32, u32> = Cache::new(100);
    let c2 = c.clone();
    let t = std::thread::spawn(move || {
        let r: Result<u32, std::sync::Arc<()>> = c2.try_get_with(1, || { std::thread::sleep(Duration::from_millis(300)); Err(()) });
        r.is_err()
    });
    std::thread::sleep(Duration::from_millis(100));
    let r = catch_unwind(AssertUnwindSafe(|| c.get_with(1, || 7)));
    match r {
        Ok(v) => println!("[unit] get_with -> {v}"),
        Err(e) => println!("[unit] get_with PANICKED {:?} {:?}", e.downcast_ref::<&str>(), e.downcast_ref::<String>()),
    }
    let _ = t.join();
}

fn main() {
    let which: HashSet<String> = std::env::args().skip(1).collect();
    let all = which.is_empty();
    std::panic::set_hook(Box::new(|i| eprintln!("  panic: {i}")));
    if all || which.contains("inv") { invalidate_scan(); }
    if all || which.contains("gen") { gen_wrap(); }
    if all || which.contains("poison") { waiter_poison(); }
    if all || which.contains("compute") { compute_hang(); }
    if all || which.contains("unit") { unit_err(); }
    std::process::exit(0);
}
