use moka::{sync::Cache, Expiry};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
// expire_after_create panic -> untracked entry.
struct P(Arc<AtomicBool>);
impl Expiry<u32,u32> for P {
    fn expire_after_create(&self, k:&u32, _:&u32, _:Instant) -> Option<Duration> { if *k==666 && self.0.load(Ordering::SeqCst) { panic!("x") } None }
}
// read-set expiry, no timer
struct R;
impl Expiry<u32,u32> for R {
    fn expire_after_create(&self, _:&u32, _:&u32, _:Instant) -> Option<Duration> { None }
    fn expire_after_read(&self, _:&u32, _:&u32, _:Instant, _:Option<Duration>, _:Instant) -> Option<Duration> { Some(Duration::from_millis(100)) }
}
fn main() {
    std::panic::set_hook(Box::new(|_| {}));
    let flag=Arc::new(AtomicBool::new(true));
    let c: Cache<u32,u32> = Cache::builder().max_capacity(10).expire_after(P(flag)).build();
    let r=std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| c.insert(666,666)));
    println!("panicked={}", r.is_err());
    for k in 0..200 { c.insert(k,k); c.run_pending_tasks(); }
    c.run_pending_tasks();
    println!("after eviction pressure: get666={:?} entry_count={} iter={}", c.get(&666), c.entry_count(), c.iter().count());
    // No timer from unrecorded read
    let c: Cache<u32,u32> = Cache::builder().max_capacity(100).expire_after(R).build();
    c.insert(1,1); c.run_pending_tasks();
    let mut n=0;
    let v = c.entry(1).or_insert_with_if(|| 2, |_| { n+=1; n==1 });
    println!("or_insert_with_if value={} replace_if calls={n}", v.value());
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(4000));
    for _ in 0..5 { c.run_pending_tasks(); }
    println!("no-timer: entry_count={} contains={}", c.entry_count(), c.contains_key(&1));
    // don't use get to set: read contains_key
    std::thread::sleep(Duration::from_millis(3000));
    for _ in 0..5 { c.run_pending_tasks(); }
    println!("no-timer later: entry_count={} contains={}", c.entry_count(), c.contains_key(&1));
    // Nested
    let (tx,rx)=std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let c: Cache<u32,u32> = Cache::new(10);
        let c2=c.clone();
        let v=c.get_with(1, move || c2.get_with(1, || 5));
        tx.send(v).ok();
    });
    println!("nested get_with: {:?}", rx.recv_timeout(Duration::from_secs(4)));
    std::process::exit(0);
}
