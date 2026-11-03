use heapless::spsc::Queue;

fn main() {
    // A queue of zero-sized items needs no memory, so any N is free.
    let mut q = Queue::<(), { usize::MAX }>::new();
    assert!(q.is_empty());
    q.enqueue(()).unwrap();
    q.enqueue(()).unwrap();
    println!("after 2 enqueues: len={} is_empty={} iter().count()={}", q.len(), q.is_empty(), q.iter().count());
    assert_eq!(q.len(), 2, "len() must count the 2 queued items");
    println!("consistent");
}
