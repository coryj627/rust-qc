use moka::sync::Cache;
use std::{panic::{catch_unwind, AssertUnwindSafe}, sync::{Arc, Barrier}, time::Duration};
fn main() {
    std::panic::set_hook(Box::new(|i| println!("  panic: {}", i)));
    // A. get_with waiting on try_get_with whose E is ()
    let c: Cache<u32, u32> = Cache::new(100);
    let b = Arc::new(Barrier::new(2));
    let (c2, b2) = (c.clone(), b.clone());
    let t = std::thread::spawn(move || {
        c2.try_get_with(1u32, || -> Result<u32, ()> { b2.wait(); std::thread::sleep(Duration::from_millis(300)); Err(()) })
    });
    b.wait();
    std::thread::sleep(Duration::from_millis(50));
    let r = catch_unwind(AssertUnwindSafe(|| c.get_with(1, || 7)));
    println!("A get_with concurrent with try_get_with::<_, ()>: {:?}", r.map_err(|_| "PANIC"));
    println!("  try_get_with: {:?}", t.join().unwrap());

    // B. weigher panic inside get_with poisons the key
    let c: Cache<u32, u32> = Cache::builder().max_capacity(100).weigher(|_, v: &u32| { if *v == 13 { panic!("bad weight") } 1 }).build();
    let r = catch_unwind(AssertUnwindSafe(|| c.get_with(1, || 13)));
    println!("B first get_with: {:?}", r.map_err(|_| "PANIC"));
    let r = catch_unwind(AssertUnwindSafe(|| c.get_with(1, || 14)));
    println!("B second get_with (good value): {:?}", r.map_err(|_| "PANIC"));
    c.insert(1, 15);
    println!("B after insert get: {:?}", c.get(&1));
    let r = catch_unwind(AssertUnwindSafe(|| c.get_with(2, || 2)));
    println!("B other key: {:?}", r.map_err(|_| "PANIC"));
    c.invalidate(&1);
    let r = catch_unwind(AssertUnwindSafe(|| c.get_with(1, || 16)));
    println!("B third get_with after invalidate: {:?}", r.map_err(|_| "PANIC"));
}
