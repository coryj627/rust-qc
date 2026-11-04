use moka::{sync::Cache, policy::Expiry};
use std::{time::{Duration, Instant}, sync::{Arc, mpsc}, thread};

struct ShortExpiry;
impl Expiry<u32,u32> for ShortExpiry {
    fn expire_after_create(&self,_:&u32,_:&u32,_:Instant)->Option<Duration>{Some(Duration::from_millis(20))}
    fn expire_after_update(&self,_:&u32,_:&u32,_:Instant,_:Option<Duration>)->Option<Duration>{None}
}
#[test]
fn refresh_expired() {
 let c=Cache::builder().max_capacity(10).expire_after(ShortExpiry).build();
 c.insert(1,1); thread::sleep(Duration::from_millis(40));
 assert_eq!(c.get_with(1,||2),2);
 let got=c.get(&1); println!("refreshed get={got:?}"); assert_eq!(got,None);
}
#[test]
fn notification_cause() {
 let (tx,rx)=mpsc::channel();
 let c=Cache::builder().max_capacity(10).expire_after(ShortExpiry).eviction_listener(move |k,v,cause| {tx.send((*k,v,cause)).unwrap();}).build();
 c.insert(1,1); c.insert(2,2); thread::sleep(Duration::from_millis(40));
 c.invalidate(&1); c.insert(2,3); c.run_pending_tasks();
 let got:Vec<_>=rx.try_iter().collect(); println!("notifications={got:?}");
 assert!(got.contains(&(1,1,moka::notification::RemovalCause::Explicit)));
 assert!(got.contains(&(2,2,moka::notification::RemovalCause::Replaced)));
}
#[test]
fn admitted_weight() {
 let c=Cache::builder().max_capacity(100).weigher(|_:&u32,v:&u32|*v).build();
 c.insert(1,60);c.run_pending_tasks(); c.insert(1,30);c.run_pending_tasks();
 println!("replacement weight={}",c.weighted_size()); assert_eq!(c.weighted_size(),30);
 c.insert(2,60);c.run_pending_tasks();assert_eq!(c.weighted_size(),90); assert_eq!(c.entry_count(),2);
}
#[test]
fn contains_does_not_refresh() {
 let c=Cache::builder().max_capacity(10).time_to_idle(Duration::from_millis(200)).build();
 c.insert(1,1); thread::sleep(Duration::from_millis(120));assert!(c.contains_key(&1));
 thread::sleep(Duration::from_millis(120));assert_eq!(c.get(&1),None);
}
#[tokio::test]
async fn deferred_invalidate_and_iterator() {
 let c=moka::future::Cache::builder().max_capacity(100).time_to_live(Duration::from_millis(200)).build();
 c.insert(1,1).await; c.run_pending_tasks().await;
 c.invalidate_all();c.insert(1,2).await;c.insert(2,2).await;c.run_pending_tasks().await;
 assert_eq!(c.get(&1).await,Some(2)); assert_eq!(c.get(&2).await,Some(2));
 let mut iter=c.iter();tokio::time::sleep(Duration::from_millis(220)).await;
 assert_eq!(iter.next(),None);assert_eq!(c.get(&1).await,None);
}
#[test]
fn unit_error_collision() {
 let c=Cache::new(100);let (started_tx,started_rx)=mpsc::channel();let (go_tx,go_rx)=mpsc::channel();
 let c1=c.clone();let a=thread::spawn(move||c1.try_get_with(1,||{started_tx.send(()).unwrap();go_rx.recv().unwrap();Err::<u32,()>(())}));
 started_rx.recv().unwrap();let c2=c.clone();let b=thread::spawn(move||c2.get_with(1,||2));
 thread::sleep(Duration::from_millis(100));go_tx.send(()).unwrap();assert!(a.join().unwrap().is_err());assert!(b.join().is_err());
}
#[tokio::test]
async fn future_collision_and_cancellation() {
 use futures_util::{poll,FutureExt};
 let c=moka::future::Cache::new(100);let(tx,rx)=tokio::sync::oneshot::channel();
 let mut a=Box::pin(c.try_get_with(1,async{rx.await.unwrap();Err::<u32,()>(())}));
 assert!(poll!(&mut a).is_pending());
 let mut b=Box::pin(std::panic::AssertUnwindSafe(c.get_with(1,async{2})).catch_unwind());
 assert!(poll!(&mut b).is_pending());tx.send(()).unwrap();assert!(a.await.is_err());assert!(b.await.is_err());
 let mut a=Box::pin(c.get_with(2,std::future::pending()));assert!(poll!(&mut a).is_pending());
 let mut b=Box::pin(c.get_with(2,async{3}));assert!(poll!(&mut b).is_pending());drop(a);
 assert_eq!(tokio::time::timeout(Duration::from_secs(1),b).await.unwrap(),3);
}
struct ClearExpiry;
impl Expiry<u32,u32> for ClearExpiry {
 fn expire_after_create(&self,_:&u32,_:&u32,_:Instant)->Option<Duration>{Some(Duration::from_secs(2))}
 fn expire_after_read(&self,_:&u32,_:&u32,_:Instant,_:Option<Duration>,_:Instant)->Option<Duration>{None}
}
#[test]
fn timer_clear() {
 let c=Cache::builder().max_capacity(10).expire_after(ClearExpiry).build();
 c.insert(1,1);c.run_pending_tasks();thread::sleep(Duration::from_millis(1200));
 println!("before read");let _=c.get(&1);println!("after read");c.run_pending_tasks();c.invalidate(&1);c.run_pending_tasks();
}

#[tokio::test(flavor="multi_thread",worker_threads=2)]
async fn compute_lost_update() {
 use std::sync::{atomic::{AtomicUsize,Ordering}, Mutex};
 let (pause_tx,pause_rx)=mpsc::channel();let(go_tx,go_rx)=mpsc::channel();let go_rx=Mutex::new(go_rx);
 let n=AtomicUsize::new(0);
 let c=moka::future::Cache::builder().max_capacity(100).weigher(move|_:&u32,v:&u32|{
  if *v==1 && n.fetch_add(1,Ordering::SeqCst)==0 {pause_tx.send(()).unwrap();go_rx.lock().unwrap().recv().unwrap();} 1
 }).build();
 c.insert(1,0).await;
 let c1=c.clone();let a=tokio::spawn(async move {c1.entry(1).and_upsert_with(|e|async move{e.unwrap().into_value()+1}).await});
 pause_rx.recv_timeout(Duration::from_secs(2)).unwrap();
 let b=c.entry(1).and_upsert_with(|e|async move {e.unwrap().into_value()+1}).await;
 assert_eq!(b.into_value(),1);go_tx.send(()).unwrap();a.await.unwrap();
 let got=c.get(&1).await;println!("counter after two serialized increments={got:?}");assert_eq!(got,Some(1));
}

struct PausingExpiry {started:mpsc::Sender<()>,go:std::sync::Mutex<mpsc::Receiver<()>>}
impl Expiry<u32,u32> for PausingExpiry {
 fn expire_after_create(&self,k:&u32,_:&u32,_:Instant)->Option<Duration>{
  if *k==1{self.started.send(()).unwrap();self.go.lock().unwrap().recv().unwrap();} None
 }
}
#[test]
fn reordered_remove() {
 let(tx,rx)=mpsc::channel();let(go_tx,go_rx)=mpsc::channel();
 let c=Cache::builder().max_capacity(1).expire_after(PausingExpiry{started:tx,go:std::sync::Mutex::new(go_rx)}).build();
 let c1=c.clone();let a=thread::spawn(move||c1.insert(1,1));rx.recv_timeout(Duration::from_secs(2)).unwrap();
 assert_eq!(c.get(&1),Some(1));c.invalidate(&1);c.run_pending_tasks();assert_eq!(c.entry_count(),0);
 go_tx.send(()).unwrap();a.join().unwrap();c.run_pending_tasks();
 println!("empty map entries={} weight={} iter={:?}",c.entry_count(),c.weighted_size(),c.iter().collect::<Vec<_>>());
 assert_eq!(c.get(&1),None);assert_eq!(c.entry_count(),1);assert_eq!(c.iter().count(),0);
 c.invalidate_all();for _ in 0..5{c.run_pending_tasks();}assert_eq!(c.entry_count(),1);
 c.insert(2,2);for _ in 0..5{c.run_pending_tasks();}println!("new key={:?} entries={}",c.get(&2),c.entry_count());
 assert_eq!(c.get(&2),None);
}

struct PausingValue {weight:u32,pause:Option<Arc<Pause>>}
struct Pause {once:std::sync::atomic::AtomicBool,started:mpsc::Sender<()>,go:std::sync::Mutex<mpsc::Receiver<()>>}
impl Clone for PausingValue {
 fn clone(&self)->Self{
  if let Some(p)=&self.pause {if p.once.swap(false,std::sync::atomic::Ordering::SeqCst){p.started.send(()).unwrap();p.go.lock().unwrap().recv().unwrap();}}
  Self{weight:self.weight,pause:self.pause.clone()}
 }
}
#[test]
fn weight_race() {
 let(tx,rx)=mpsc::channel();let(go_tx,go_rx)=mpsc::channel();
 let pause=Arc::new(Pause{once:std::sync::atomic::AtomicBool::new(true),started:tx,go:std::sync::Mutex::new(go_rx)});
 let c=Cache::builder().max_capacity(100).weigher(|_:&u32,v:&PausingValue|v.weight).build();
 c.insert(0,PausingValue{weight:10,pause:None});c.insert(1,PausingValue{weight:1,pause:None});c.run_pending_tasks();
 let c1=c.clone();let a=thread::spawn(move||c1.insert(1,PausingValue{weight:3,pause:Some(pause)}));
 rx.recv_timeout(Duration::from_secs(2)).unwrap();c.insert(1,PausingValue{weight:2,pause:None});
 go_tx.send(()).unwrap();a.join().unwrap();c.run_pending_tasks();
 println!("actual weight={} accounted={}",c.iter().map(|(_,v)|v.weight).sum::<u32>(),c.weighted_size());assert_eq!(c.weighted_size(),12);
 c.invalidate(&1);c.run_pending_tasks();println!("remaining actual=10 accounted={}",c.weighted_size());assert_eq!(c.weighted_size(),9);
}

#[test]
fn invalidate_all_race() {
 use std::sync::atomic::{AtomicBool,Ordering};
 let c=Cache::new(100);let stop=Arc::new(AtomicBool::new(false));let mut handles=vec![];
 for _ in 0..3{let c=c.clone();let stop=stop.clone();handles.push(thread::spawn(move||{while !stop.load(Ordering::Relaxed){c.invalidate_all();}}));}
 let mut seen=None;
 for i in 0..100_000 {c.insert(1,i);c.invalidate_all();if let Some(v)=c.get(&1){seen=Some((i,v));break;}}
 stop.store(true,Ordering::Relaxed);for h in handles{h.join().unwrap();}
 println!("invalidated entry reappeared={seen:?}");assert!(seen.is_some());
}

#[test]
fn generation_wrap() {
 fn probe(n:u32)->(u64,u64) {
  let c=Cache::builder().max_capacity(10).time_to_live(Duration::from_secs(3)).build();
  c.insert(1,0);c.run_pending_tasks();for i in 0..n{c.insert(1,i);c.run_pending_tasks();}
  thread::sleep(Duration::from_millis(3100));assert_eq!(c.get(&1),None);for _ in 0..5 {c.run_pending_tasks();}
  (c.entry_count(),c.weighted_size())
 }
 let before=probe(65534);let after=probe(65535);println!("before wrap={before:?}; after={after:?}");assert_eq!(before,(0,0));assert_eq!(after,(1,1));
}
