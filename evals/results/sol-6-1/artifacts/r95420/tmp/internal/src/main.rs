#![allow(dead_code, unexpected_cfgs)]
#[path = "/work/crate/src/cht/map/bucket.rs"]
mod bucket;
use bucket::{BucketArray, InsertOrModifyState, InsertionResult};
use std::sync::{Arc, Barrier};
use std::sync::atomic::Ordering;
fn main() {
    let buckets = Arc::new(BucketArray::<u32, Arc<()>>::with_length(0, 128));
    let barrier = Arc::new(Barrier::new(2));
    let value = Arc::new(());
    let threads: Vec<_> = (0..2).map(|_| {
        let buckets = buckets.clone();
        let barrier = barrier.clone();
        let value = value.clone();
        std::thread::spawn(move || {
            let guard = crossbeam_epoch::pin();
            let state = InsertOrModifyState::New(1, || {
                barrier.wait();
                value
            });
            match buckets.insert_if_not_present(&guard, 0, state) {
                Ok(InsertionResult::Inserted) => println!("inserted"),
                Ok(InsertionResult::AlreadyPresent(_)) => println!("lost CAS, then found existing"),
                _ => panic!("unexpected result"),
            }
        })
    }).collect();
    for t in threads { t.join().unwrap(); }
    let guard = unsafe { crossbeam_epoch::unprotected() };
    let ptr = buckets.buckets[0].swap(crossbeam_epoch::Shared::null(), Ordering::Relaxed, guard);
    unsafe { bucket::defer_destroy_bucket(guard, ptr); }
    drop(buckets);
    println!("strong_count after destroying buckets = {}", Arc::strong_count(&value));
    assert_eq!(Arc::strong_count(&value), 1, "losing insertion leaked its value");
}
