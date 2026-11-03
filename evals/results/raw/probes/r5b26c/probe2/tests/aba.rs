use heapless::mpmc::Queue;
use std::sync::atomic::{AtomicU8, AtomicU64, AtomicUsize, Ordering::*};
use std::time::{Duration, Instant};
const PER: usize = 1 << 25; // values per producer
const T: usize = 12;
static Q: Queue<u64, 2> = Queue::new();
static CNT: AtomicUsize = AtomicUsize::new(0);
static DONE: AtomicUsize = AtomicUsize::new(0);
static SENT: [AtomicU64; T] = [const { AtomicU64::new(0) }; T];
#[test] fn aba() {
    let secs: u64 = std::env::var("SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(100);
    let mut v = std::vec::Vec::with_capacity(T);
    for _ in 0..T { let mut x: std::vec::Vec<AtomicU8> = std::vec::Vec::with_capacity(PER); unsafe { std::ptr::write_bytes(x.as_mut_ptr(), 0, PER); x.set_len(PER); } v.push(x); }
    let seen = &v;
    let start = Instant::now(); let limit = Duration::from_secs(secs); let hard = limit + Duration::from_secs(15);
    let stuck = AtomicUsize::new(0); let stuck=&stuck;
    std::thread::scope(|s| {
        for t in 0..T { s.spawn(move || {
            let mut i = 0usize;
            'o: while start.elapsed() < limit && i < PER {
                let val = ((t as u64) << 40) | i as u64;
                let mut tries = 0u32;
                loop { if Q.enqueue(val).is_ok() { SENT[t].store(i as u64 + 1, Relaxed); i += 1; break; }
                    tries += 1; if tries % 1024 == 0 && start.elapsed() > hard { stuck.fetch_add(1, Relaxed); break 'o; } std::hint::spin_loop(); }
            }
            DONE.fetch_add(1, SeqCst);
        }); }
        for _ in 0..T { s.spawn(move || {
            let mut idle = 0u32;
            loop {
                if let Some(val) = Q.dequeue() { let t=(val>>40) as usize; let i=(val & ((1<<40)-1)) as usize; seen[t][i].fetch_add(1, Relaxed); CNT.fetch_add(1, Relaxed); idle=0; }
                else { idle+=1; if DONE.load(SeqCst)>=T && idle>100_000 { break; } if idle % 1024==0 && start.elapsed()>hard { break; } std::hint::spin_loop(); }
            }
        }); }
    });
    // single-threaded drain of whatever is left
    let mut left = 0; while let Some(val) = Q.dequeue() { let t=(val>>40) as usize; let i=(val & ((1<<40)-1)) as usize; seen[t][i].fetch_add(1, Relaxed); left += 1; if left>10 {break;} }
    let (mut lost, mut dup, mut sent) = (0u64, 0u64, 0u64);
    let mut ex = std::vec::Vec::new();
    for t in 0..T { let n=SENT[t].load(Relaxed) as usize; sent+=n as u64; for i in 0..n { match seen[t][i].load(Relaxed) { 1=>{}, 0=>{lost+=1; if ex.len()<6 {ex.push((t,i,0));}}, c=>{dup+=(c-1) as u64; if ex.len()<6 {ex.push((t,i,c));}} } } }
    eprintln!("sent {sent} delivered {} left {left} lost {lost} duplicated {dup} stuck_producers {} elapsed {:?} examples {ex:?}", CNT.load(Relaxed), stuck.load(Relaxed), start.elapsed());
    assert_eq!((lost,dup),(0,0));
}
