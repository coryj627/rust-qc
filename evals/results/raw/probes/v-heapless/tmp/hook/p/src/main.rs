
use heapless::mpmc::{self, Queue};
static QE: Queue<u32,2> = Queue::new();
static QD: Queue<Box<u32>,2> = Queue::new();
fn he() {
    for i in 0..254u32 { QE.enqueue(i).unwrap(); QE.dequeue().unwrap(); }
    QE.enqueue(1000).unwrap(); QE.enqueue(1001).unwrap();
    println!("hook: queue full; enqueue(5000) -> {:?}", QE.enqueue(5000));
}
fn hd() {
    // outer dequeue saw pos=0 with item 0 ready
    let x = QD.dequeue().unwrap(); println!("hook took {}", x); drop(x);
    for i in 0..254u32 { QD.enqueue(Box::new(i)).unwrap(); QD.dequeue().unwrap(); }
    // dequeue_pos now 255; advance one more leaving empty cell 0 with dequeue_pos 0 after wrap
    QD.enqueue(Box::new(1)).unwrap(); QD.dequeue().unwrap();
}
fn main() {
    match std::env::args().nth(1).unwrap().as_str() {
        "enq" => {
            unsafe { mpmc::HOOK_ENQ = Some(he); }
            println!("outer enqueue(7) -> {:?}", QE.enqueue(7));
            std::thread::spawn(|| { println!("dequeue -> {:?}", QE.dequeue()); println!("dequeue -> {:?}", QE.dequeue()); println!("dequeue -> {:?}", QE.dequeue()); });
            std::thread::sleep(std::time::Duration::from_secs(5));
            println!("after 5s: main exits");
        }
        "deq" => {
            QD.enqueue(Box::new(42)).unwrap();
            unsafe { mpmc::HOOK_DEQ = Some(hd); }
            let v = QD.dequeue();
            println!("outer dequeue -> {:?}", v);
            println!("next dequeue -> {:?}", QD.dequeue());
        }
        _ => panic!(),
    }
}
