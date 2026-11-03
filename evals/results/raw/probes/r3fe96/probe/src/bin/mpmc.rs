use heapless::mpmc::Queue;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::*};
use std::time::{Duration, Instant};
static Q: Queue<u64, 2> = Queue::new();
static SENT: AtomicU64 = AtomicU64::new(0);
static RECV: AtomicU64 = AtomicU64::new(0);
static SENTN: AtomicU64 = AtomicU64::new(0);
static RECVN: AtomicU64 = AtomicU64::new(0);
static STOP: AtomicBool = AtomicBool::new(false);
fn main() {
    let secs: u64 = std::env::args().nth(1).unwrap().parse().unwrap();
    let nthreads: u64 = std::env::args().nth(2).unwrap().parse().unwrap();
    for t in 0..nthreads {
        std::thread::spawn(move || {
            let mut c = t << 40;
            let mut held: Option<u64> = None;
            while !STOP.load(Relaxed) {
                c += 1;
                let v = held.take().unwrap_or(c);
                match Q.enqueue(v) {
                    Ok(()) => { SENT.fetch_add(v, Relaxed); SENTN.fetch_add(1, Relaxed); }
                    Err(v) => held = Some(v),
                }
                if let Some(x) = Q.dequeue() { RECV.fetch_add(x, Relaxed); RECVN.fetch_add(1, Relaxed); }
            }
        });
    }
    let start = Instant::now();
    let mut last = 0; let mut stuck = 0;
    while start.elapsed() < Duration::from_secs(secs) {
        std::thread::sleep(Duration::from_secs(1));
        let n = RECVN.load(Relaxed);
        if n == last { stuck += 1; if stuck >= 5 { println!("NO PROGRESS for 5s at {} ops: every thread spinning", n); println!("sent {} recv {}", SENTN.load(Relaxed), RECVN.load(Relaxed)); std::process::exit(3); } } else { stuck = 0; }
        last = n;
    }
    STOP.store(true, Relaxed);
    std::thread::sleep(Duration::from_millis(500));
    while let Some(x) = Q.dequeue() { RECV.fetch_add(x, Relaxed); RECVN.fetch_add(1, Relaxed); }
    println!("ops sent {} recv {} sum equal {}", SENTN.load(Relaxed), RECVN.load(Relaxed), SENT.load(Relaxed) == RECV.load(Relaxed));
}
