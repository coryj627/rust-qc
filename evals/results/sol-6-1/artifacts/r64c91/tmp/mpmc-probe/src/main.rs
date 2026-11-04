use heapless::mpmc::{Queue, AUDIT_SCHEDULE};
use std::sync::atomic::Ordering;
fn main() {
    let q = Queue::<u16, 2>::new();
    std::thread::scope(|scope| {
        let q_ref = &q;
        let paused = scope.spawn(move || q_ref.enqueue(999));
        while AUDIT_SCHEDULE.load(Ordering::SeqCst) != 1 { std::thread::yield_now(); }
        for i in 0..254 {
            q.enqueue(i).unwrap();
            assert_eq!(q.dequeue(), Some(i));
        }
        q.enqueue(254).unwrap();
        q.enqueue(255).unwrap();
        AUDIT_SCHEDULE.store(2, Ordering::SeqCst);
        let result = paused.join().unwrap();
        println!("paused enqueue on full queue: {result:?}");
        // The bug also prevents Queue::drop from finishing because dequeue spins forever.
        std::mem::forget(q);
        assert_eq!(result, Err(999));
    });
}
