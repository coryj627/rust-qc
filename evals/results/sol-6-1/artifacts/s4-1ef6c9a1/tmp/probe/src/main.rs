use std::sync::{Arc, atomic::{AtomicBool, Ordering}, mpsc, Mutex};
use moka::{future::Cache, ops::compute::Op};

#[tokio::main(flavor="multi_thread", worker_threads=2)]
async fn main() {
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let release_rx = Mutex::new(release_rx);
    let first = AtomicBool::new(true);
    let cache = Cache::builder().max_capacity(100).weigher(move |_: &u32, v: &u32| {
        if *v == 1 && first.swap(false, Ordering::SeqCst) {
            entered_tx.send(()).unwrap();
            release_rx.lock().unwrap().recv().unwrap();
        }
        1
    }).build();
    cache.insert(0, 0).await;
    let c = cache.clone();
    let task = tokio::spawn(async move {
        c.entry(0).and_compute_with(|e| async move { Op::Put(e.unwrap().into_value() + 1) }).await
    });
    entered_rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
    let seen = Arc::new(AtomicBool::new(false));
    let seen2 = seen.clone();
    let second = cache.entry(0).and_compute_with(move |e| {
        let value = e.unwrap().into_value();
        println!("second compute observed {value}");
        seen2.store(true, Ordering::SeqCst);
        async move { Op::Put(value + 1) }
    }).await;
    println!("second result: {second:?}");
    release_tx.send(()).unwrap();
    task.await.unwrap();
    let result = cache.get(&0).await.unwrap();
    println!("final value = {result}, expected 2");
    assert_eq!(result, 2, "serialized compute must preserve both increments");
}
