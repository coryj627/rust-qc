use moka::future::Cache as ACache;
use moka::sync::Cache;
use moka::Expiry;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

struct E(Duration);
impl<K,V> Expiry<K,V> for E { fn expire_after_create(&self,_:&K,_:&V,_:Instant)->Option<Duration>{Some(self.0)}
   fn expire_after_update(&self,_:&K,_:&V,_:Instant,_:Option<Duration>)->Option<Duration>{Some(self.0)} }

fn causes() {
    let ev = Arc::new(Mutex::new(vec![])); let e2 = ev.clone();
    let c: Cache<u32,u32> = Cache::builder().expire_after(E(Duration::from_millis(100))).eviction_listener(move |k,v,cause| e2.lock().unwrap().push((*k,v,cause))).build();
    c.insert(1,1); c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(150));
    c.insert(1,2); c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(150));
    c.invalidate(&1);
    println!("[cause] per-entry expired then overwritten then invalidated: {:?}", ev.lock().unwrap());
}

fn sync_pred_panic() {
    let hits = Arc::new(AtomicU64::new(0));
    let c: Cache<u32, u32> = Cache::builder().support_invalidation_closures().build();
    for i in 0..10 { c.insert(i, i); }
    c.run_pending_tasks();
    let h = hits.clone();
    c.invalidate_entries_if(move |k, _| { if h.fetch_add(1, Ordering::SeqCst) == 0 { panic!("once") } *k < 5 }).unwrap();
    let r = catch_unwind(AssertUnwindSafe(|| c.run_pending_tasks()));
    println!("[sync-pred] first run panicked={}", r.is_err());
    let r = catch_unwind(AssertUnwindSafe(|| { c.run_pending_tasks(); c.run_pending_tasks(); }));
    println!("[sync-pred] later runs panicked={} entry_count={} real={}", r.is_err(), c.entry_count(), c.iter().count());
}

fn stall() {
    for round in 0..25 {
        let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(4).enable_all().build().unwrap();
        let hung = rt.block_on(async {
            let done = Arc::new(AtomicU64::new(0));
            let c: ACache<u64, u64> = ACache::new(100);
            let tasks = 32u64;
            for t in 0..tasks {
                let c = c.clone(); let done = done.clone();
                tokio::spawn(async move {
                    for i in 0..4000u64 { c.insert(i.wrapping_mul(t + 7) % 500, i).await; }
                    done.fetch_add(1, Ordering::SeqCst);
                });
            }
            let start = Instant::now();
            loop {
                tokio::time::sleep(Duration::from_millis(50)).await;
                if done.load(Ordering::SeqCst) == tasks { return None; }
                if start.elapsed() > Duration::from_secs(8) { return Some(done.load(Ordering::SeqCst)); }
            }
        });
        if let Some(d) = hung { println!("[stall] round {round}: only {d} of 32 tasks finished within 8s"); std::process::exit(0); }
        rt.shutdown_background();
    }
    println!("[stall] no stall in 25 rounds");
}

fn main() {
    std::panic::set_hook(Box::new(|i| eprintln!("  panic: {}", i.to_string().replace('\n', " | "))));
    causes();
    sync_pred_panic();
    stall();
    std::process::exit(0);
}
