use std::{future::Future, sync::{Arc, atomic::{AtomicBool, Ordering}, mpsc}, task::{Wake, Waker, Context}, time::Duration};
fn block_on<F: Future>(f: F) -> F::Output {
    struct W(std::thread::Thread);
    impl Wake for W { fn wake(self: Arc<Self>) { self.0.unpark(); } }
    let w = Waker::from(Arc::new(W(std::thread::current())));
    let mut cx = Context::from_waker(&w);
    let mut f = Box::pin(f);
    loop { match f.as_mut().poll(&mut cx) { std::task::Poll::Ready(v) => return v, std::task::Poll::Pending => std::thread::park() } }
}
fn main() {
    let armed = Arc::new(AtomicBool::new(false));
    let (started_tx, started_rx) = mpsc::channel();
    let (resume_tx, resume_rx) = mpsc::channel();
    let resume_rx = std::sync::Mutex::new(resume_rx);
    let a = armed.clone();
    let cache = moka::future::Cache::builder().max_capacity(100).weigher(move |_: &u32, _: &u32| {
        if a.swap(false, Ordering::SeqCst) { started_tx.send(()).unwrap(); resume_rx.lock().unwrap().recv().unwrap(); }
        1
    }).build();
    block_on(cache.insert(0, 0));
    armed.store(true, Ordering::SeqCst);
    let c1 = cache.clone();
    let t1 = std::thread::spawn(move || block_on(c1.entry(0).and_upsert_with(|e| async move { e.unwrap().into_value() + 1 })));
    started_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    let c2 = cache.clone();
    let (result_tx, result_rx) = mpsc::channel();
    let t2 = std::thread::spawn(move || { let e = block_on(c2.entry(0).and_upsert_with(|e| async move { e.unwrap().into_value() + 1 })); result_tx.send(e.into_value()).unwrap(); });
    println!("second compute while first insert paused: {:?}", result_rx.recv_timeout(Duration::from_secs(1)));
    resume_tx.send(()).unwrap();
    t1.join().unwrap(); t2.join().unwrap();
    println!("final counter {:?}, expected Some(2)", block_on(cache.get(&0)));
}
