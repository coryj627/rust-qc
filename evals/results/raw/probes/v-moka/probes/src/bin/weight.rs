use moka::sync::Cache;
use std::thread;
use std::time::{Duration, Instant};
fn drain(c:&Cache<u32,u32>){ for _ in 0..5 { c.run_pending_tasks(); } }
fn main() {
    // on_modify drift: key pre-inserted & settled, no first-insert race, weights change.
    for round in 0..3 {
        let c: Cache<u32,u32> = Cache::builder().max_capacity(1<<40).weigher(|_, v: &u32| *v).build();
        c.insert(1, 5); drain(&c);
        let end = Instant::now()+Duration::from_secs(3);
        let mut hs=vec![];
        for t in 0..4u32 { let c=c.clone(); hs.push(thread::spawn(move || {
            let mut i=t; while Instant::now()<end { c.insert(1, 1 + (i % 1000)); i=i.wrapping_add(13); }
        })); }
        for h in hs {h.join().unwrap();}
        c.insert(1, 5); drain(&c);
        println!("round {round}: update race, one key final weight 5: weighted_size={} entry_count={}", c.weighted_size(), c.entry_count());
    }
    // same-weight first-insert race: weights all 1, no removes.
    let c: Cache<u32,u32> = Cache::builder().build();
    let n = 300000u32;
    let mut hs=vec![];
    for _ in 0..4 { let c=c.clone(); hs.push(thread::spawn(move || { for k in 0..n { c.insert(k,k);} })); }
    for h in hs {h.join().unwrap();}
    drain(&c);
    println!("first-insert race, unweighted: entries={} entry_count={} weighted_size={}", c.iter().count(), c.entry_count(), c.weighted_size());
    // same but sequential control
    let c: Cache<u32,u32> = Cache::builder().build();
    for _ in 0..4 { for k in 0..n { c.insert(k,k);} }
    drain(&c);
    println!("control sequential: entries={} entry_count={} weighted_size={}", c.iter().count(), c.entry_count(), c.weighted_size());
}
