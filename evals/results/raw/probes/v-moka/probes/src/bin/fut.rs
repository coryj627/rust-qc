use moka::future::Cache;
use moka::Expiry;
use std::time::{Duration, Instant};
struct E;
impl Expiry<u32, u32> for E {
    fn expire_after_create(&self, _: &u32, _: &u32, _: Instant) -> Option<Duration> { Some(Duration::from_millis(500)) }
    fn expire_after_read(&self, _: &u32, _: &u32, _: Instant, _: Option<Duration>, _: Instant) -> Option<Duration> { None }
}
#[tokio::main(flavor="multi_thread", worker_threads=4)]
async fn main() {
    // phantom
    let c: Cache<u32,u32> = Cache::new(1000);
    let end = Instant::now()+Duration::from_secs(5);
    let mut hs=vec![];
    for t in 0..4u32 { let c=c.clone(); hs.push(tokio::spawn(async move {
        let mut i = t * 2654435761 + 1;
        while Instant::now()<end { i ^= i << 13; i ^= i >> 17; i ^= i << 5; let k=(i>>8)%4; if i&1==0 { c.insert(k,i).await; } else { c.invalidate(&k).await; } }
    })); }
    for h in hs {h.await.unwrap();}
    for k in 0..4 {c.invalidate(&k).await;}
    for _ in 0..5 {c.run_pending_tasks().await;}
    println!("future phantom: entry_count={} real={}", c.entry_count(), c.iter().count());
    for k in 10000..10500 {c.insert(k,k).await;}
    for _ in 0..5 {c.run_pending_tasks().await;}
    println!("future retained {}", c.iter().count());
    // inv
    let c: Cache<u32,u32> = Cache::builder().max_capacity(100000).support_invalidation_closures().build();
    for k in 0..2000 { c.insert(k,k).await; }
    c.run_pending_tasks().await;
    tokio::time::sleep(Duration::from_millis(5)).await;
    c.invalidate_entries_if(|_,_| true).unwrap();
    for _ in 0..200 {c.run_pending_tasks().await;}
    println!("future inv survivors {:?}", (0..2000).filter(|k| c.contains_key(k)).collect::<Vec<_>>());
    // gen
    let c: Cache<u32,u32> = Cache::builder().max_capacity(100).time_to_live(Duration::from_millis(100)).build();
    for i in 0..70000 { c.insert(1,i).await; if i%100==0 {c.run_pending_tasks().await;} }
    c.run_pending_tasks().await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    for _ in 0..5 {c.run_pending_tasks().await;}
    println!("future gen entry_count={} get={:?}", c.entry_count(), c.get(&1).await);
    // remove
    let c: Cache<u32,u32> = Cache::builder().max_capacity(10).time_to_live(Duration::from_millis(50)).build();
    c.insert(1,1).await; tokio::time::sleep(Duration::from_millis(100)).await;
    println!("future remove after TTL get={:?} remove={:?}", c.get(&1).await, c.remove(&1).await);
    // replaced
    let c: Cache<u32,u32> = Cache::new(10);
    c.insert(1,1).await;
    let e = c.entry(1).or_insert_with_if(async {2}, |_| true).await;
    println!("future or_insert_with_if value={} fresh={} replaced={}", e.value(), e.is_fresh(), e.is_old_value_replaced());
    // byref
    let c: Cache<String,u32> = Cache::new(10);
    c.insert("a".into(),1).await;
    let e=c.entry_by_ref("a").or_try_insert_with(async {Ok::<u32,()>(2)}).await.unwrap();
    println!("future byref key {}", e.key());
    // uaf
    let c: Cache<u32,u32> = Cache::builder().max_capacity(100).expire_after(E).build();
    c.insert(1,1).await;
    c.run_pending_tasks().await;
    let mut n=0;
    let v=c.entry(1).or_insert_with_if(async {2}, |_| {n+=1; n==1}).await;
    println!("future uaf: value={} calls={n}", v.value());
    c.run_pending_tasks().await;
    tokio::time::sleep(Duration::from_millis(3500)).await;
    c.run_pending_tasks().await;
    let junk: Vec<Vec<u64>> = (0..2000).map(|i| vec![0xdead_beef_dead_beef + i; 6]).collect();
    println!("junk {}", junk.len());
    c.invalidate(&1).await;
    c.run_pending_tasks().await;
    println!("future uaf: done");
}
