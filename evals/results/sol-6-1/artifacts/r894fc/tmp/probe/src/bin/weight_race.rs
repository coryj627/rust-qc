use std::sync::{Arc, Mutex, mpsc, atomic::{AtomicBool,Ordering}};
use std::time::Duration;
struct Gate { armed:AtomicBool, started:mpsc::Sender<()>, resume:Mutex<mpsc::Receiver<()>> }
struct V { weight:u32, gate:Option<Arc<Gate>> }
impl Clone for V {
    fn clone(&self)->Self {
        if let Some(g)=&self.gate { if g.armed.swap(false,Ordering::SeqCst) { g.started.send(()).unwrap();g.resume.lock().unwrap().recv().unwrap(); } }
        Self {weight:self.weight,gate:self.gate.clone()}
    }
}
fn main() {
    let (tx,rx)=mpsc::channel();let (tx2,rx2)=mpsc::channel();
    let g=Arc::new(Gate {armed:AtomicBool::new(true), started:tx,resume:Mutex::new(rx2)});
    let c = moka::sync::Cache::builder().max_capacity(100).weigher(|_:&u32,v:&V|v.weight).build();
    c.insert(0,V{weight:1,gate:None});c.insert(1,V{weight:10,gate:None});c.run_pending_tasks();
    let c1=c.clone();let t=std::thread::spawn(move ||c1.insert(0,V{weight:3,gate:Some(g)}));
    rx.recv_timeout(Duration::from_secs(1)).unwrap();
    c.insert(0,V{weight:2,gate:None});c.run_pending_tasks();
    println!("before resuming weight {}",c.weighted_size());
    tx2.send(()).unwrap();t.join().unwrap();c.run_pending_tasks();
    println!("stored weight {} plus other weight 10, reported weight {}",c.get(&0).unwrap().weight,c.weighted_size());
    c.remove(&0);c.run_pending_tasks();
    println!("after remove reported weight {}",c.weighted_size());
}
