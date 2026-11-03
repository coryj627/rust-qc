use moka::sync::Cache;
use moka::policy::EvictionPolicy;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
fn lock_step(c: Cache<u32,u32>, n: u32) {
    let gate = Arc::new(AtomicUsize::new(0));
    let mut hs=vec![];
    for _ in 0..4 { let (c,g)=(c.clone(),gate.clone()); hs.push(std::thread::spawn(move || {
        for k in 0..n {
            g.fetch_add(1, Ordering::SeqCst);
            while g.load(Ordering::SeqCst) < (k as usize+1)*4 { std::hint::spin_loop(); }
            c.insert(k,k);
        }
    })); }
    for h in hs {h.join().unwrap();}
}
fn main() {
    let n=300000;
    let c: Cache<u32,u32> = Cache::builder().build();
    lock_step(c.clone(), n);
    for _ in 0..5 {c.run_pending_tasks();}
    println!("unbounded: entries={} entry_count={} weighted={}", c.iter().count(), c.entry_count(), c.weighted_size());
    let c: Cache<u32,u32> = Cache::builder().max_capacity(5000).eviction_policy(EvictionPolicy::lru()).build();
    lock_step(c.clone(), n);
    for _ in 0..5 {c.run_pending_tasks();}
    println!("lru 5000: entries={} entry_count={} weighted={}", c.iter().count(), c.entry_count(), c.weighted_size());
    let c: Cache<u32,u32> = Cache::builder().max_capacity(5000).build();
    lock_step(c.clone(), n);
    for _ in 0..5 {c.run_pending_tasks();}
    println!("tinylfu 5000: entries={} entry_count={} weighted={}", c.iter().count(), c.entry_count(), c.weighted_size());
}
