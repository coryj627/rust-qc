use std::{sync::{Arc, Mutex, mpsc}, time::{Duration, Instant}};
struct Exp { started: mpsc::Sender<()>, resume: Mutex<mpsc::Receiver<()>> }
impl moka::Expiry<u32,u32> for Exp {
    fn expire_after_create(&self, key: &u32, _: &u32, _: Instant) -> Option<Duration> {
        if *key == 0 { self.started.send(()).unwrap(); self.resume.lock().unwrap().recv().unwrap(); } None
    }
}
fn main() {
    let (started_tx, started_rx) = mpsc::channel(); let (resume_tx,resume_rx) = mpsc::channel();
    let c = moka::sync::Cache::builder().max_capacity(1).expire_after(Exp { started:started_tx, resume:Mutex::new(resume_rx) }).build();
    let c1 = c.clone(); let t = std::thread::spawn(move || c1.insert(0,1));
    started_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    println!("remove {:?}",c.remove(&0));
    c.run_pending_tasks();
    println!("after remove count {}", c.entry_count());
    resume_tx.send(()).unwrap(); t.join().unwrap();
    for _ in 0..3 { c.run_pending_tasks(); }
    println!("after delayed insertion log get {:?}, iter count {}, entry count {}", c.get(&0), c.iter().count(),c.entry_count());
    c.invalidate_all(); c.run_pending_tasks();
    println!("after invalidate_all count {}", c.entry_count());
    c.insert(1,2); c.run_pending_tasks(); println!("new key {:?}, count {}",c.get(&1),c.entry_count());
    let _ = Arc::new(());
}
