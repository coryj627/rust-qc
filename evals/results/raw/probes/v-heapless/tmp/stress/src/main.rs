use heapless::mpmc::Queue;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering::*};
use std::time::{Duration, Instant};
static Q: Queue<u64,2> = Queue::new();
static STOP: AtomicBool = AtomicBool::new(false);
static PDONE: AtomicU64 = AtomicU64::new(0);
static DUP: AtomicU64 = AtomicU64::new(0);
static CONSUMED: AtomicU64 = AtomicU64::new(0);
const P: u64 = 12; const BITS: u64 = 26;
fn main() {
    let secs: u64 = std::env::args().nth(1).unwrap().parse().unwrap();
    let seen: &'static Vec<Vec<AtomicU8>> = Box::leak(Box::new((0..P).map(|_| (0..(1usize<<(BITS-3))).map(|_| AtomicU8::new(0)).collect()).collect()));
    let prod: &'static Vec<AtomicU64> = Box::leak(Box::new((0..P).map(|_| AtomicU64::new(0)).collect()));
    let lastprog: &'static Vec<AtomicU64> = prod;
    for p in 0..P { std::thread::spawn(move || {
        let mut seq=0u64;
        while !STOP.load(Relaxed) && seq < (1<<BITS) {
            let v=(p<<48)|seq;
            loop { if STOP.load(Relaxed) {break} if Q.enqueue(v).is_ok() { seq+=1; prod[p as usize].store(seq,Relaxed); break; } std::hint::spin_loop(); }
        }
        PDONE.fetch_add(1,SeqCst);
    }); }
    for _ in 0..P { std::thread::spawn(move || { loop {
        if let Some(v)=Q.dequeue() {
            let p=(v>>48) as usize; let s=v&((1<<48)-1);
            if p>=P as usize || s>=(1<<BITS) { DUP.fetch_add(1,SeqCst); eprintln!("garbage {v:#x}"); continue; }
            let m=1u8<<(s&7);
            if seen[p][(s>>3) as usize].fetch_or(m,SeqCst)&m!=0 { DUP.fetch_add(1,SeqCst); eprintln!("duplicate p{p} s{s}"); }
            CONSUMED.fetch_add(1,Relaxed);
        } else { std::hint::spin_loop(); if STOP.load(Relaxed) && PDONE.load(SeqCst)==P { break; } }
    }}); }
    let t=Instant::now();
    let mut last=0; let mut stalled=0;
    while t.elapsed().as_secs()<secs {
        std::thread::sleep(Duration::from_secs(1));
        let c=CONSUMED.load(Relaxed);
        if c==last { stalled+=1; if stalled>=5 { println!("no progress for 5s at {}s",t.elapsed().as_secs()); break; } } else { stalled=0; }
        last=c;
    }
    STOP.store(true,SeqCst);
    std::thread::sleep(Duration::from_secs(5));
    let produced: u64 = lastprog.iter().map(|x|x.load(Relaxed)).sum();
    let consumed=CONSUMED.load(SeqCst);
    let mut lost=0u64;
    if PDONE.load(SeqCst)==P { /* all producers finished */ }
    for p in 0..P as usize { let n=prod[p].load(Relaxed); for s in 0..n { if seen[p][(s>>3) as usize].load(Relaxed)&(1u8<<(s&7))==0 { lost+=1; } } }
    println!("elapsed {}s produced {} consumed {} dup {} missing {} producers_exited {}/{}", t.elapsed().as_secs(), produced, consumed, DUP.load(SeqCst), lost, PDONE.load(SeqCst), P);
    std::process::exit(0);
}
