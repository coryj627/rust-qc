use std::{sync::{Arc, Barrier}, time::{Duration, Instant}};
use moka::{sync::{Cache, SegmentedCache}, Expiry, ops::compute::{Op, CompResult}};
use futures_util::FutureExt;

#[test]
fn borrowed_sync_key() {
 let c=Cache::new(10); c.insert("key".to_string(),42);
 let e=c.entry_by_ref("key").or_try_insert_with(||Ok::<_,()>(99)).unwrap();
 assert_eq!(*e.value(),42); assert_eq!(e.key(),"key");
}
#[test]
fn borrowed_segmented_key() {
 let c=SegmentedCache::new(10,2); c.insert("key".to_string(),42);
 let e=c.entry_by_ref("key").or_try_insert_with(||Ok::<_,()>(99)).unwrap();
 assert_eq!(*e.value(),42); assert_eq!(e.key(),"key");
}
#[tokio::test]
async fn borrowed_future_key_control() {
 let c=moka::future::Cache::new(10); c.insert("key".to_string(),42).await;
 let e=c.entry_by_ref("key").or_try_insert_with(async{Ok::<_,()>(99)}).await.unwrap();
 assert_eq!(*e.value(),42); assert_eq!(e.key(),"key");
}
#[test]
fn sync_unit_error_collision() {
 let c=Cache::<u8,u8>::new(10); let start=Arc::new(Barrier::new(2)); let finish=Arc::new(Barrier::new(2));
 let a=c.clone(); let s=start.clone(); let f=finish.clone();
 let fallible=std::thread::spawn(move||a.try_get_with(1,||{s.wait(); f.wait(); Err(())}));
 start.wait();
 let b=c.clone(); let infallible=std::thread::spawn(move||b.get_with(1,||7));
 std::thread::sleep(Duration::from_millis(150)); finish.wait();
 assert!(fallible.join().unwrap().is_err());
 assert_eq!(infallible.join().unwrap(),7);
}
#[tokio::test]
async fn future_unit_error_collision() {
 let c=moka::future::Cache::<u8,u8>::new(10);
 let (tx,rx)=tokio::sync::oneshot::channel::<()>();
 let mut a=Box::pin(c.try_get_with(1,async{rx.await.unwrap(); Err::<u8,()>(())}));
 assert!(futures_util::poll!(&mut a).is_pending());
 let mut b=Box::pin(std::panic::AssertUnwindSafe(c.get_with(1,async{7})).catch_unwind());
 assert!(futures_util::poll!(&mut b).is_pending());
 tx.send(()).unwrap(); assert!(a.await.is_err());
 assert_eq!(b.await.unwrap(),7);
}
#[tokio::test]
async fn cancelled_loader_control() {
 let c=moka::future::Cache::<u8,u8>::new(10);
 let mut first=Box::pin(c.get_with(1,std::future::pending()));
 assert!(futures_util::poll!(&mut first).is_pending()); drop(first);
 assert_eq!(tokio::time::timeout(Duration::from_secs(1), c.get_with(1,async{8})).await.unwrap(),8);
}
#[test]
fn failed_compute_preserves_control() {
 let c=Cache::new(10); c.insert(1,7);
 let r=c.entry(1).and_try_compute_with(|e|{assert_eq!(e.unwrap().into_value(),7); Err::<Op<i32>,_>(())});
 assert!(r.is_err()); assert_eq!(c.get(&1),Some(7));
}
struct ClearExpiry;
impl Expiry<u8,u8> for ClearExpiry {
 fn expire_after_create(&self,_:&u8,_:&u8,_:Instant)->Option<Duration>{Some(Duration::from_millis(10))}
 fn expire_after_update(&self,_:&u8,_:&u8,_:Instant,_:Option<Duration>)->Option<Duration>{None}
}
#[test]
fn sync_clear_elapsed_expiry() {
 let c=Cache::builder().expire_after(ClearExpiry).build(); c.insert(1,1);
 std::thread::sleep(Duration::from_millis(30)); c.insert(1,2); assert_eq!(c.get(&1),Some(2));
}
#[tokio::test]
async fn future_clear_elapsed_expiry() {
 let c=moka::future::Cache::builder().expire_after(ClearExpiry).build(); c.insert(1,1).await;
 tokio::time::sleep(Duration::from_millis(30)).await; c.insert(1,2).await; assert_eq!(c.get(&1).await,Some(2));
}
#[test]
fn sync_remove_invalidated() {
 let c=Cache::new(10); c.insert(1,42); c.invalidate_all(); assert_eq!(c.get(&1),None); assert_eq!(c.remove(&1),None);
}
#[tokio::test]
async fn future_remove_invalidated() {
 let c=moka::future::Cache::new(10); c.insert(1,42).await; c.invalidate_all(); assert_eq!(c.get(&1).await,None); assert_eq!(c.remove(&1).await,None);
}
#[test]
fn sync_compute_remove_invalidated() {
 let c=Cache::new(10); c.insert(1,42); c.invalidate_all();
 let r=c.entry(1).and_compute_with(|e|{assert!(e.is_none());Op::Remove});
 assert!(matches!(r,CompResult::StillNone(_)),"{r:?}");
}
#[tokio::test]
async fn future_compute_remove_invalidated() {
 let c=moka::future::Cache::new(10); c.insert(1,42).await; c.invalidate_all();
 let r=c.entry(1).and_compute_with(|e|async move{assert!(e.is_none());Op::Remove}).await;
 assert!(matches!(r,CompResult::StillNone(_)),"{r:?}");
}
#[test]
fn sync_replacement_flag() {
 let c=Cache::new(10); c.insert(1,10);
 let e=c.entry(1).or_insert_with_if(||20,|v|*v==10);
 assert_eq!(c.get(&1),Some(20)); assert_eq!(*e.value(),20); assert!(e.is_old_value_replaced());
}
#[tokio::test]
async fn future_replacement_flag() {
 let c=moka::future::Cache::new(10); c.insert(1,10).await;
 let e=c.entry(1).or_insert_with_if(async{20},|v|*v==10).await;
 assert_eq!(c.get(&1).await,Some(20)); assert_eq!(*e.value(),20); assert!(e.is_old_value_replaced());
}
#[tokio::test]
async fn async_compute_serialization() {
 let entered=Arc::new(tokio::sync::Notify::new()); let release=Arc::new(tokio::sync::Notify::new());
 let en=entered.clone(); let rel=release.clone();
 let c=moka::future::Cache::builder().max_capacity(10).async_eviction_listener(move|_,v,_| {
  let en=en.clone(); let rel=rel.clone();
  async move {if v==0 {en.notify_one(); rel.notified().await;}}.boxed()
 }).build();
 c.insert(1,0).await;
 let mut a=Box::pin(c.entry(1).and_upsert_with(|e|async move{e.unwrap().into_value()+1}));
 assert!(futures_util::poll!(&mut a).is_pending()); entered.notified().await;
 let mut b=Box::pin(c.entry(1).and_upsert_with(|e|async move{e.unwrap().into_value()+1}));
 assert!(futures_util::poll!(&mut b).is_pending());
 let mut d=Box::pin(c.entry(1).and_upsert_with(|e|async move{e.unwrap().into_value()+1}));
 assert!(futures_util::poll!(&mut d).is_pending());
 release.notify_one(); let (av,bv,dv)=tokio::join!(a,b,d);
 println!("compute values: {},{},{}",av.value(),bv.value(),dv.value());
 assert_eq!(c.get(&1).await,Some(3));
}
struct PausingExpiry { entered:Arc<Barrier>, release:Arc<Barrier> }
impl Expiry<u8,u32> for PausingExpiry {
 fn expire_after_update(&self,_:&u8,v:&u32,_:Instant,d:Option<Duration>)->Option<Duration>{
  if *v==2 {self.entered.wait(); self.release.wait();} d
 }
}
#[test]
fn sync_weight_reorder() {
 let entered=Arc::new(Barrier::new(2)); let release=Arc::new(Barrier::new(2));
 let c=Cache::builder().max_capacity(100).weigher(|_:&u8,v:&u32|*v)
  .expire_after(PausingExpiry{entered:entered.clone(),release:release.clone()}).build();
 c.insert(1,1); c.run_pending_tasks(); assert_eq!(c.weighted_size(),1);
 let c2=c.clone(); let t=std::thread::spawn(move||c2.insert(1,2));
 entered.wait(); c.insert(1,3); release.wait(); t.join().unwrap(); c.run_pending_tasks();
 assert_eq!(c.get(&1),Some(3)); println!("sync weight {}",c.weighted_size());
 c.remove(&1); c.run_pending_tasks(); println!("sync empty weight {}",c.weighted_size());
 assert_eq!(c.weighted_size(),0);
}
#[tokio::test]
async fn future_weight_reorder() {
 let entered=Arc::new(Barrier::new(2)); let release=Arc::new(Barrier::new(2));
 let c=moka::future::Cache::builder().max_capacity(100).weigher(|_:&u8,v:&u32|*v)
  .expire_after(PausingExpiry{entered:entered.clone(),release:release.clone()}).build();
 c.insert(1,1).await; c.run_pending_tasks().await; assert_eq!(c.weighted_size(),1);
 let c2=c.clone(); let t=std::thread::spawn(move||tokio::runtime::Builder::new_current_thread().build().unwrap().block_on(c2.insert(1,2)));
 entered.wait(); c.insert(1,3).await; release.wait(); t.join().unwrap(); c.run_pending_tasks().await;
 assert_eq!(c.get(&1).await,Some(3)); println!("future weight {}",c.weighted_size());
 c.remove(&1).await; c.run_pending_tasks().await; println!("future empty weight {}",c.weighted_size());
 assert_eq!(c.weighted_size(),0);
}
