use moka::{sync::Cache, future::Cache as FC};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
#[tokio::main(flavor="multi_thread", worker_threads=2)]
async fn main() {
    std::panic::set_hook(Box::new(|_| {}));
    // sync
    let c: Cache<u32,u32> = Cache::builder().max_capacity(100).support_invalidation_closures().build();
    for k in 0..10 { c.insert(k,k); }
    let b=Arc::new(AtomicBool::new(true)); let b2=b.clone();
    c.invalidate_entries_if(move |_,_| { if b2.swap(false,Ordering::SeqCst) { panic!("pred") } false }).unwrap();
    let r=std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| c.run_pending_tasks()));
    println!("sync 1st rpt panicked={}", r.is_err());
    let r=std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {c.run_pending_tasks();c.run_pending_tasks();}));
    println!("sync 2nd rpt panicked={} entry_count={} map={}", r.is_err(), c.entry_count(), c.iter().count());
    // future
    let c: FC<u32,u32> = FC::builder().max_capacity(100).support_invalidation_closures().build();
    for k in 0..10 { c.insert(k,k).await; }
    let b=Arc::new(AtomicBool::new(true)); let b2=b.clone();
    c.invalidate_entries_if(move |_,_| { if b2.swap(false,Ordering::SeqCst) { panic!("pred") } false }).unwrap();
    let cc=c.clone();
    let r=tokio::spawn(async move { cc.run_pending_tasks().await }).await;
    println!("future 1st rpt panicked={}", r.is_err());
    for i in 0..2 { let cc=c.clone();
      let r=tokio::spawn(async move { cc.run_pending_tasks().await }).await;
      println!("future rpt {i} panicked={}", r.is_err());
    }
    let cc=c.clone();
    let r=tokio::spawn(async move { tokio::time::sleep(std::time::Duration::from_millis(400)).await; for k in 100..200 { cc.insert(k,k).await; let _=cc.get(&k).await; } }).await;
    println!("future inserts after 400ms panicked={}", r.is_err());
    std::process::exit(0);
}
