use std::{sync::{Arc, atomic::{AtomicBool, Ordering}}, time::{Duration, Instant}};
fn main() {
    let c = moka::sync::Cache::new(100);
    let stop=Arc::new(AtomicBool::new(false));
    let mut ts=Vec::new();
    for _ in 0..6 {
        let c=c.clone(); let stop=stop.clone();
        ts.push(std::thread::spawn(move || while !stop.load(Ordering::Relaxed) { c.invalidate_all(); }));
    }
    let start=Instant::now(); let mut it=0; let mut bad=0;
    while start.elapsed() < Duration::from_secs(3) {
        c.insert(0, it); c.invalidate_all();
        if let Some(v)=c.get(&0) { println!("invalidated entry reappeared {v} at iteration {it}"); bad+=1; break; }
        it+=1;
    }
    stop.store(true,Ordering::Relaxed);
    for t in ts {t.join().unwrap();}
    println!("{it} iterations, {bad} failures");
}
