use heapless::spsc::Queue;
use std::thread;
#[test] fn peek_race() {
    let mut q: Queue<u64, 4> = Queue::new();
    let (mut p, mut c) = q.split();
    thread::scope(|s| {
        s.spawn(move || { for i in 0..20u64 { while p.enqueue(i).is_err() { thread::yield_now(); } } });
        s.spawn(move || {
            let mut n = 0;
            while n < 20 {
                if let Some(v) = c.peek() { assert_eq!(*v, n); c.dequeue().unwrap(); n += 1; } else { thread::yield_now(); }
            }
        });
    });
}
#[test] fn ready_dequeue_unchecked_race() {
    let mut q: Queue<u64, 4> = Queue::new();
    let (mut p, mut c) = q.split();
    thread::scope(|s| {
        s.spawn(move || { for i in 0..20u64 { while p.enqueue(i).is_err() { thread::yield_now(); } } });
        s.spawn(move || {
            let mut n = 0;
            while n < 20 {
                if c.ready() { let v = unsafe { c.dequeue_unchecked() }; assert_eq!(v, n); n += 1; } else { thread::yield_now(); }
            }
        });
    });
}
#[test] fn ready_enqueue_unchecked_race() {
    let mut q: Queue<u64, 4> = Queue::new();
    let (mut p, mut c) = q.split();
    thread::scope(|s| {
        s.spawn(move || { for i in 0..20u64 { loop { if p.ready() { unsafe { p.enqueue_unchecked(i) }; break; } thread::yield_now(); } } });
        s.spawn(move || {
            let mut n = 0;
            while n < 20 {
                if let Some(v) = c.dequeue() { assert_eq!(v, n); n += 1; } else { thread::yield_now(); }
            }
        });
    });
}
