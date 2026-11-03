use heapless::spsc::Queue;
#[test]
fn spsc_consumer_peek_race() {
    let q: &'static mut Queue<u64, 4> = Box::leak(Box::new(Queue::new()));
    let (mut p, c) = q.split();
    let t = std::thread::spawn(move || { let _ = p.enqueue(42); });
    let u = std::thread::spawn(move || { loop { if let Some(v) = c.peek() { assert_eq!(*v, 42); break; } std::thread::yield_now(); } });
    t.join().unwrap(); u.join().unwrap();
}
