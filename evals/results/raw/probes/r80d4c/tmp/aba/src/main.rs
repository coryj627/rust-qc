use heapless::mpmc::{self, Queue};
static Q: Queue<u32, 2> = Queue::new();
fn main() {
    // An "interrupt" runs between enqueue's load of the position and its CAS.
    mpmc::set_hook(Box::new(|| {
        for i in 0..254 { Q.enqueue(i).unwrap(); assert_eq!(Q.dequeue(), Some(i)); }
        Q.enqueue(1000).unwrap();
        Q.enqueue(1001).unwrap();
        assert!(Q.enqueue(1002).is_err(), "queue full inside the interrupt");
    }));
    println!("outer enqueue(7) -> {:?}", Q.enqueue(7));
    let t = std::thread::spawn(|| { for _ in 0..4 { println!("dequeue -> {:?}", Q.dequeue()); } });
    std::thread::sleep(std::time::Duration::from_secs(5));
    println!("dequeue thread finished: {}", t.is_finished());
    std::process::exit(0);
}
