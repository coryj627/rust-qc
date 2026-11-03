// A panic in a user callback outside init (weigher / replace_if / Expiry) during get_with / upsert.
use moka::sync::Cache;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
fn main() {
    std::panic::set_hook(Box::new(|i| eprintln!("PANIC: {}", i)));
    let boom = Arc::new(AtomicBool::new(true));
    let b = boom.clone();
    let cache: Cache<u32, u32> = Cache::builder().max_capacity(100).weigher(move |_, _| { if b.swap(false, Ordering::SeqCst) { panic!("weigher once") } 1 }).build();
    let r = catch_unwind(AssertUnwindSafe(|| cache.get_with(1, || 10)));
    println!("first get_with (weigher panics): err={}", r.is_err());
    for i in 0..2 {
        let r = catch_unwind(AssertUnwindSafe(|| cache.get_with(1, || 10)));
        println!("retry {i} get_with on same key, weigher now fine: {:?}", r.map_err(|_| "panicked"));
    }
    println!("other key OK: {}", cache.get_with(2, || 20));

    // replace_if panic
    let c2: Cache<u32, u32> = Cache::new(100);
    c2.insert(1, 1);
    let mut n = 0;
    let r = catch_unwind(AssertUnwindSafe(|| c2.entry(1).or_insert_with_if(|| 5, |_| { n += 1; if n == 2 { panic!("replace_if") } true })));
    println!("or_insert_with_if panic: err={}", r.is_err());
    let r = catch_unwind(AssertUnwindSafe(|| c2.entry(1).or_insert_with_if(|| 5, |_| true).into_value()));
    println!("next or_insert_with_if: {:?}", r.map_err(|_| "panicked"));

    // upsert: weigher panic -> does the next compute hang?
    let boom = Arc::new(AtomicBool::new(true));
    let b = boom.clone();
    let c3: Cache<u32, u32> = Cache::builder().max_capacity(100).weigher(move |_, _| { if b.swap(false, Ordering::SeqCst) { panic!("weigher once") } 1 }).build();
    let r = catch_unwind(AssertUnwindSafe(|| c3.entry(1).and_upsert_with(|_| 1).into_value()));
    println!("first upsert: err={}", r.is_err());
    let c = c3.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || { let v = c.entry(1).and_upsert_with(|_| 2).into_value(); tx.send(v).unwrap(); });
    println!("second upsert within 5s: {:?}", rx.recv_timeout(Duration::from_secs(5)));
}
