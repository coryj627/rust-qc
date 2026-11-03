use std::{sync::{Arc, atomic::{AtomicBool, Ordering}}, time::{Duration, Instant}};
fn main() {
    let c = moka::sync::Cache::builder().max_capacity(100).support_invalidation_closures().build();
    let stop=Arc::new(AtomicBool::new(false));
    let c2=c.clone(); let s=stop.clone();
    let t=std::thread::spawn(move || while !s.load(Ordering::Relaxed) { c2.run_pending_tasks(); });
    let start=Instant::now(); let mut it=0; let mut bad=0;
    while start.elapsed() < Duration::from_secs(5) {
        c.insert(0, it); c.invalidate_entries_if(|_,_| true).unwrap();
        if let Some(v)=c.get(&0) { println!("invalidated entry reappeared {v} at iteration {it}"); bad+=1; break; }
        it+=1;
    }
    stop.store(true,Ordering::Relaxed);t.join().unwrap();
    println!("{it} iterations, {bad} failures");
}
