use heapless::mpmc::Queue;
use heapless::{box_pool, arc_pool, pool::boxed::BoxBlock, pool::arc::ArcBlock};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering::*};
struct R(u64);
impl R { fn n(&mut self, m: usize) -> usize { let mut x=self.0; x^=x<<13; x^=x>>7; x^=x<<17; self.0=x; (x % m as u64) as usize } }
fn model<const N: usize>(cap: usize) {
    for seed in 1..50u64 { let mut r=R(seed*7+1); let q:Queue<u32,N>=Queue::new(); let mut m=VecDeque::new();
      for i in 0..3000u32 { if r.n(2)==0 { let f=m.len()==cap; assert_eq!(q.enqueue(i).is_err(),f,"seed {seed} i {i} len {}",m.len()); if !f {m.push_back(i);} } else { assert_eq!(q.dequeue(),m.pop_front()); } }
    }
}
#[test] fn mpmc_model() { model::<2>(2); model::<4>(4); model::<64>(64); model::<128>(128); }
#[test] fn mpmc_stress_u8() {
    // conservation check: each value delivered exactly once
    static Q: Queue<u64, 2> = Queue::new();
    static SENT: AtomicU64 = AtomicU64::new(0);
    static RECV: AtomicU64 = AtomicU64::new(0);
    static CNT: AtomicUsize = AtomicUsize::new(0);
    const PER: u64 = 4_000_000;
    std::thread::scope(|s| {
        for t in 0..2u64 { s.spawn(move || { for i in 0..PER { let v = (t << 40) | i; let mut spins=0u64; while Q.enqueue(v).is_err() { spins+=1; if spins>200_000_000 { eprintln!("enqueue stuck"); std::process::exit(9);} std::hint::spin_loop(); } SENT.fetch_add(v, Relaxed); } }); }
        for _ in 0..2 { s.spawn(|| { let mut idle=0u64; loop { if let Some(v)=Q.dequeue() { RECV.fetch_add(v, Relaxed); CNT.fetch_add(1, Relaxed); idle=0; } else { idle+=1; if CNT.load(Relaxed) as u64 >= 2*PER { break; } if idle>300_000_000 { eprintln!("dequeue stuck at {}", CNT.load(Relaxed)); break; } std::hint::spin_loop(); } } }); }
    });
    eprintln!("count {} sent {} recv {}", CNT.load(Relaxed), SENT.load(Relaxed), RECV.load(Relaxed));
    assert_eq!(SENT.load(Relaxed), RECV.load(Relaxed));
}
box_pool!(BP: u64);
#[test] fn pool_race() {
    use std::ptr::addr_of_mut;
    static mut B: [BoxBlock<u64>; 2] = [const { BoxBlock::new() }; 2];
    let bs = unsafe { addr_of_mut!(B).as_mut().unwrap() };
    for b in bs { BP.manage(b); }
    std::thread::scope(|s| { for t in 0..3u64 { s.spawn(move || { for i in 0..30 { if let Ok(mut b)=BP.alloc(t*i) { *b += 1; std::thread::yield_now(); drop(b); } std::thread::yield_now(); } }); } });
}
arc_pool!(AP: u64);
#[test] fn arc_basic() {
    use std::ptr::addr_of_mut;
    static mut B: ArcBlock<u64> = ArcBlock::new();
    AP.manage(unsafe { addr_of_mut!(B).as_mut().unwrap() });
    let a = AP.alloc(1).unwrap(); let b=a.clone(); assert_eq!(*b,1); assert!(AP.alloc(2).is_err()); drop(a); assert!(AP.alloc(2).is_err()); drop(b); assert_eq!(*AP.alloc(3).unwrap(),3);
}
#[test] fn mpmc_aba_oversubscribed() {
    static Q: Queue<u64, 2> = Queue::new();
    static SENT: AtomicU64 = AtomicU64::new(0);
    static RECV: AtomicU64 = AtomicU64::new(0);
    static CNT: AtomicUsize = AtomicUsize::new(0);
    static DONE: AtomicUsize = AtomicUsize::new(0);
    let start = std::time::Instant::now();
    let limit = std::time::Duration::from_secs(150);
    std::thread::scope(|s| {
        for t in 0..8u64 { s.spawn(move || { let mut i=0u64; while start.elapsed() < limit { let v=(t<<40)|i; i+=1; loop { if Q.enqueue(v).is_ok() {SENT.fetch_add(v,Relaxed); break;} if start.elapsed()>limit+std::time::Duration::from_secs(10) { eprintln!("producer gave up: queue never drains; count {}", CNT.load(Relaxed)); DONE.fetch_add(1,Relaxed); return; } std::thread::yield_now(); } } DONE.fetch_add(1,Relaxed); }); }
        for _ in 0..8 { s.spawn(|| { loop { if let Some(v)=Q.dequeue() { RECV.fetch_add(v,Relaxed); CNT.fetch_add(1,Relaxed); } else { if DONE.load(Relaxed)>=8 { if Q.dequeue().is_none() { break; } } if start.elapsed()>limit+std::time::Duration::from_secs(20) {break;} std::thread::yield_now(); } } }); }
    });
    eprintln!("count {} sent {} recv {} diff {}", CNT.load(Relaxed), SENT.load(Relaxed), RECV.load(Relaxed), SENT.load(Relaxed) as i128 - RECV.load(Relaxed) as i128);
    assert_eq!(SENT.load(Relaxed), RECV.load(Relaxed));
}
