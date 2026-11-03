use heapless::mpmc::{Queue, HOOK};
static Q: Queue<u32, 2> = Queue::new();
fn preempter() {
    // another context does 254 enqueue/dequeue pairs then fills the queue:
    // enqueue_pos has wrapped (mod 256) back to the value the stalled caller loaded.
    for i in 0..254 { Q.enqueue(i).unwrap(); assert_eq!(Q.dequeue(), Some(i)); }
    Q.enqueue(1000).unwrap();
    Q.enqueue(1001).unwrap();
    println!("queue full: third enqueue is_err = {}", Q.enqueue(1002).is_err());
}
fn main() {
    unsafe { HOOK = Some(preempter); }
    println!("stalled enqueue(777) on a FULL queue returned {:?}", Q.enqueue(777));
    let h = std::thread::spawn(|| {
        for _ in 0..3 { println!("dequeue -> {:?}", Q.dequeue()); }
        println!("consumer finished");
    });
    std::thread::sleep(std::time::Duration::from_secs(5));
    println!("consumer finished after 5s: {}", h.is_finished());
    std::process::exit(0);
}
