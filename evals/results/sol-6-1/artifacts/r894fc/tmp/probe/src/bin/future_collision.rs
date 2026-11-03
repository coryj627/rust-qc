#[path="../support/mod.rs"] mod support;
use support::block_on;
use std::{sync::mpsc, time::Duration};
fn main() {
    let c = moka::future::Cache::<u32,u32>::new(100);
    let (started_tx, started_rx) = mpsc::channel(); let (resume_tx,resume_rx) = mpsc::channel();
    let c1=c.clone(); let t1=std::thread::spawn(move || block_on(c1.try_get_with(0, async { started_tx.send(()).unwrap(); resume_rx.recv().unwrap(); Err::<u32,()>(()) })));
    started_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    let c2=c.clone(); let (entered_tx,entered_rx)=mpsc::channel();
    let t2=std::thread::spawn(move || { entered_tx.send(()).unwrap(); block_on(c2.get_with(0,async {123})) });
    entered_rx.recv_timeout(Duration::from_secs(1)).unwrap();std::thread::sleep(Duration::from_millis(50));resume_tx.send(()).unwrap();
    println!("try_get_with {:?}",t1.join()); println!("get_with panicked = {}",t2.join().is_err());
}
