use heapless::mpmc::{Queue, PREEMPT_HOOK};
static Q: Queue<std::boxed::Box<u32>, 2> = Queue::new();
fn other_context() {
    let a = Q.dequeue().unwrap();
    println!("preempting context got {a}");
    for i in 0..255u32 { Q.enqueue(Box::new(i)).ok().unwrap(); drop(Q.dequeue().unwrap()); }
}
#[test]
fn stale_dequeue_after_256_ops() {
    Q.enqueue(Box::new(7)).ok().unwrap();
    unsafe { PREEMPT_HOOK = Some(other_context); }
    let r = Q.dequeue();
    println!("preempted dequeue returned {:?}", r.as_ref().map(|b| **b));
}
