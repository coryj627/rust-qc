use moka::{future, sync, policy::{Expiry, EvictionPolicy}, ops::compute::Op};
use std::{sync::{Arc, Mutex, atomic::{AtomicBool, AtomicUsize, Ordering}, mpsc}, time::{Duration, Instant}, panic::{catch_unwind, AssertUnwindSafe}};
use futures_util::FutureExt;

#[test]
fn sync_borrowed_hit_key() {
    let c = sync::Cache::<String,u32>::new(10);
    c.insert("key".into(), 7);
    let e = c.entry_by_ref("key").or_try_insert_with(|| Ok::<_,()>(8)).unwrap();
    assert_eq!(e.key(), "key");
}
#[test]
fn segmented_borrowed_hit_key() {
    let c = sync::SegmentedCache::<String,u32>::new(10, 4);
    c.insert("key".into(), 7);
    let e = c.entry_by_ref("key").or_try_insert_with(|| Ok::<_,()>(8)).unwrap();
    assert_eq!(e.key(), "key");
}
#[tokio::test]
async fn async_borrowed_hit_key_control() {
    let c = future::Cache::<String,u32>::new(10);
    c.insert("key".into(), 7).await;
    let e = c.entry_by_ref("key").or_try_insert_with(async { Ok::<_,()>(8) }).await.unwrap();
    assert_eq!(e.key(), "key");
}
struct ClearExpired;
impl Expiry<u32,u32> for ClearExpired {
    fn expire_after_create(&self, _: &u32, _: &u32, _: Instant) -> Option<Duration> { Some(Duration::ZERO) }
    fn expire_after_update(&self, _: &u32, _: &u32, _: Instant, _: Option<Duration>) -> Option<Duration> { None }
}
#[test]
fn sync_clear_expired_insert() {
    let c = sync::Cache::builder().max_capacity(10).expire_after(ClearExpired).build();
    c.insert(1,10); c.insert(1,20);
    assert_eq!(c.get(&1), Some(20));
}
#[test]
fn sync_clear_expired_get_with() {
    let c = sync::Cache::builder().max_capacity(10).expire_after(ClearExpired).build();
    c.insert(1,10); assert_eq!(c.get_with(1,||20),20);
    assert_eq!(c.get(&1), Some(20));
}
#[tokio::test]
async fn async_clear_expired_insert() {
    let c = future::Cache::builder().max_capacity(10).expire_after(ClearExpired).build();
    c.insert(1,10).await; c.insert(1,20).await;
    assert_eq!(c.get(&1).await, Some(20));
}
#[tokio::test]
async fn async_clear_expired_get_with() {
    let c = future::Cache::builder().max_capacity(10).expire_after(ClearExpired).build();
    c.insert(1,10).await; assert_eq!(c.get_with(1,async {20}).await,20);
    assert_eq!(c.get(&1).await, Some(20));
}
#[test]
fn sync_retry_after_weigher_panic() {
    let once = AtomicBool::new(true);
    let c = sync::Cache::builder().max_capacity(10).weigher(move |_:&u32,_:&u32| {
        if once.swap(false,Ordering::SeqCst) { panic!("one weigher panic"); } 1
    }).build();
    assert!(catch_unwind(AssertUnwindSafe(|| c.get_with(1,||7))).is_err());
    let retry = catch_unwind(AssertUnwindSafe(|| c.get_with(1,||8)));
    assert_eq!(retry.ok(),Some(8));
}
#[tokio::test]
async fn async_retry_after_weigher_panic_control() {
    let once = AtomicBool::new(true);
    let c = future::Cache::builder().max_capacity(10).weigher(move |_:&u32,_:&u32| {
        if once.swap(false,Ordering::SeqCst) { panic!("one weigher panic"); } 1
    }).build();
    assert!(AssertUnwindSafe(c.get_with(1,async {7})).catch_unwind().await.is_err());
    assert_eq!(c.get_with(1,async {8}).await,8);
}
#[tokio::test]
async fn async_mixed_unit_initializers() {
    let c = future::Cache::<u32,u32>::new(10);
    let (tx,rx) = tokio::sync::oneshot::channel::<()>();
    let mut fallible = Box::pin(c.try_get_with(1,async {rx.await.unwrap(); Err::<u32,()>(())}));
    assert!(futures_util::poll!(fallible.as_mut()).is_pending());
    let mut infallible = Box::pin(c.get_with(1,async {9}));
    assert!(futures_util::poll!(infallible.as_mut()).is_pending());
    tx.send(()).unwrap(); assert!(fallible.await.is_err());
    assert_eq!(infallible.await,9);
}
#[test]
fn sync_mixed_unit_initializers() {
    let c = sync::Cache::<u32,u32>::new(10);
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let c1 = c.clone();
    let first = std::thread::spawn(move || c1.try_get_with(1,||{started_tx.send(()).unwrap(); release_rx.recv_timeout(Duration::from_secs(5)).unwrap(); Err::<u32,()>(())}));
    started_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let c2 = c.clone(); let (tx,rx) = mpsc::channel();
    let second = std::thread::spawn(move || {tx.send(()).unwrap(); c2.get_with(1,||9)});
    rx.recv_timeout(Duration::from_secs(5)).unwrap();
    std::thread::sleep(Duration::from_millis(150)); release_tx.send(()).unwrap();
    assert!(first.join().unwrap().is_err());
    assert_eq!(second.join().ok(),Some(9));
}
#[tokio::test]
async fn cancelled_initializer_control() {
    let c = future::Cache::<u32,u32>::new(10);
    let mut f = Box::pin(c.get_with(1,std::future::pending()));
    assert!(futures_util::poll!(f.as_mut()).is_pending()); drop(f);
    assert_eq!(tokio::time::timeout(Duration::from_secs(1),c.get_with(1,async {8})).await.unwrap(),8);
}
#[test]
fn failed_compute_preserves_value_control() {
    let c = sync::Cache::<u32,u32>::new(10); c.insert(1,7);
    let r = c.entry(1).and_try_compute_with(|e| {assert_eq!(e.unwrap().into_value(),7); Err::<Op<u32>,()>(())});
    assert!(r.is_err()); assert_eq!(c.get(&1),Some(7));
}
#[tokio::test(flavor="multi_thread",worker_threads=4)]
async fn async_upserts_serialize_through_write() {
    let n = AtomicUsize::new(0);
    let (tx,rx) = mpsc::channel(); let (release_tx,release_rx) = mpsc::channel();
    let release_rx = Mutex::new(release_rx);
    let c = future::Cache::builder().max_capacity(10).weigher(move |_:&u32,v:&u32| {
        if *v==1 && n.fetch_add(1,Ordering::SeqCst)==0 {
            tx.send(()).unwrap(); release_rx.lock().unwrap().recv_timeout(Duration::from_secs(5)).unwrap();
        } 1
    }).build();
    c.insert(1,0).await;
    let c1=c.clone();
    let first=tokio::spawn(async move {c1.entry(1).and_upsert_with(|e| async move {e.unwrap().into_value()+1}).await});
    rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let c2=c.clone();
    let second=tokio::spawn(async move {c2.entry(1).and_upsert_with(|e| async move {e.unwrap().into_value()+1}).await});
    // Give the second operation time to encounter the computation lock. A correct
    // implementation waits there; this implementation completes the second write.
    tokio::time::sleep(Duration::from_millis(150)).await;
    let finished_before_release=second.is_finished();
    release_tx.send(()).unwrap(); first.await.unwrap(); second.await.unwrap();
    eprintln!("second finished before first commit: {finished_before_release}");
    assert_eq!(c.get(&1).await,Some(2));
}
#[test]
fn sync_generation_boundary() {
    for writes in [65535,65536] {
        let c = sync::Cache::builder().max_capacity(1).eviction_policy(EvictionPolicy::lru()).build();
        for i in 0..writes {c.insert(1,i); c.run_pending_tasks();}
        c.insert(2,999); c.run_pending_tasks();
        eprintln!("writes={writes}, key1={:?}, key2={:?}",c.get(&1),c.get(&2));
        assert_eq!(c.get(&1),None,"writes={writes}"); assert_eq!(c.get(&2),Some(999));
    }
}
#[tokio::test]
async fn async_generation_boundary() {
    let c = future::Cache::builder().max_capacity(1).eviction_policy(EvictionPolicy::lru()).build();
    for i in 0..65536 {c.insert(1,i).await; c.run_pending_tasks().await;}
    c.insert(2,999).await; c.run_pending_tasks().await;
    assert_eq!(c.get(&1).await,None); assert_eq!(c.get(&2).await,Some(999));
}
#[test]
fn iterator_filters_expired_control() {
    let c = sync::Cache::builder().time_to_live(Duration::from_millis(80)).build(); c.insert(1,7);
    let mut before = c.iter(); std::thread::sleep(Duration::from_millis(120));
    assert_eq!(before.next(),None); assert_eq!(c.iter().next(),None);
}
#[test]
fn invalidate_all_immediate_control() {
    let c = sync::Cache::<u32,u32>::new(10); c.insert(1,7);
    std::thread::sleep(Duration::from_millis(2)); c.invalidate_all();
    assert_eq!(c.get(&1),None); assert!(!c.contains_key(&1));
}
#[test]
fn contains_does_not_refresh_idle_control() {
    let c = sync::Cache::builder().time_to_idle(Duration::from_millis(200)).build(); c.insert(1,7);
    std::thread::sleep(Duration::from_millis(140)); assert!(c.contains_key(&1));
    std::thread::sleep(Duration::from_millis(100)); assert_eq!(c.get(&1),None);
}
