use moka::sync::Cache;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
fn with_timeout<F: FnOnce()->String + Send + 'static>(name:&str, f:F) {
    let (tx,rx)=std::sync::mpsc::channel();
    std::thread::spawn(move || { let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)); let _=tx.send(r); });
    match rx.recv_timeout(Duration::from_secs(5)) {
        Ok(Ok(s))=>println!("{name}: ok {s}"),
        Ok(Err(p))=>println!("{name}: PANIC {:?}", p.downcast_ref::<String>().map(|s|&s[..60.min(s.len())])),
        Err(_)=>println!("{name}: HANG >5s"),
    }
}
fn main() {
    std::panic::set_hook(Box::new(|_| {}));
    let boom = Arc::new(AtomicBool::new(true));
    let b = boom.clone();
    let c: Cache<u32,u32> = Cache::builder().max_capacity(100).weigher(move |_, _| { if b.swap(false, Ordering::SeqCst) { panic!("weigher") } 1 }).build();
    let c1=c.clone();
    with_timeout("get_with with panicking weigher", move || { c1.get_with(1, || 1).to_string() });
    let c2=c.clone();
    with_timeout("get_with again 1", move || { c2.get_with(1, || 1).to_string() });
    let c2=c.clone();
    with_timeout("get_with again 2", move || { c2.get_with(1, || 1).to_string() });
    let c2=c.clone();
    with_timeout("get_with other key", move || { c2.get_with(2, || 1).to_string() });

    let boom = Arc::new(AtomicBool::new(true));
    let b = boom.clone();
    let c: Cache<u32,u32> = Cache::builder().max_capacity(100).weigher(move |_, _| { if b.swap(false, Ordering::SeqCst) { panic!("weigher") } 1 }).build();
    let c1=c.clone();
    with_timeout("and_upsert_with panicking weigher", move || { c1.entry(1).and_upsert_with(|_| 1).into_value().to_string() });
    let c1=c.clone();
    with_timeout("and_upsert_with again", move || { c1.entry(1).and_upsert_with(|_| 1).into_value().to_string() });
    std::process::exit(0);
}
