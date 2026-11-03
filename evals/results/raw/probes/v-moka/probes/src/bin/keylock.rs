use moka::{sync::Cache, notification::RemovalCause};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
fn main() {
    let in_prog: Arc<Vec<AtomicU32>> = Arc::new((0..8).map(|_| AtomicU32::new(0)).collect());
    let overlaps = Arc::new(AtomicU64::new(0));
    let calls = Arc::new(AtomicU64::new(0));
    let (ip, ov, ca) = (in_prog.clone(), overlaps.clone(), calls.clone());
    let c: Cache<u32,u32> = Cache::builder().max_capacity(1000).eviction_listener(move |k: Arc<u32>, _, _: RemovalCause| {
        ca.fetch_add(1, Ordering::Relaxed);
        let i=&ip[*k as usize];
        if i.fetch_add(1, Ordering::SeqCst) != 0 { ov.fetch_add(1, Ordering::Relaxed); }
        for _ in 0..200 { std::hint::spin_loop(); }
        i.fetch_sub(1, Ordering::SeqCst);
    }).build();
    let end=Instant::now()+Duration::from_secs(12);
    let mut hs=vec![];
    for t in 0..4u32 { let c=c.clone(); hs.push(std::thread::spawn(move || {
        let mut i=t; while Instant::now()<end { c.insert(i%8, i); i=i.wrapping_add(1+t); }
    })); }
    for h in hs {h.join().unwrap();}
    println!("listener calls={} overlaps={}", calls.load(Ordering::Relaxed), overlaps.load(Ordering::Relaxed));
}
