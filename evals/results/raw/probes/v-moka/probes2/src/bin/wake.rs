use moka::future::Cache;
use futures_util::FutureExt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
#[tokio::main(flavor="multi_thread", worker_threads=8)]
async fn main() {
    for round in 0..60 {
        let c: Cache<u32,u32> = Cache::builder().max_capacity(100)
            .async_eviction_listener(|k,_v,_c| async move { if *k % 8 == 0 { tokio::time::sleep(Duration::from_millis(1)).await } }.boxed()).build();
        let done=Arc::new(AtomicU64::new(0));
        let prog=Arc::new(AtomicU64::new(0));
        let mut hs=vec![];
        for t in 0..32u32 { let (c,d,p)=(c.clone(),done.clone(),prog.clone()); hs.push(tokio::spawn(async move {
            for i in 0..500u32 { c.insert(t*10000+i, i).await; p.fetch_add(1,Ordering::SeqCst); }
            d.fetch_add(1,Ordering::SeqCst);
        })); }
        let mut last=0; let mut idle=0;
        while done.load(Ordering::SeqCst)<32 {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let p=prog.load(Ordering::SeqCst);
            if p==last { idle+=1; } else { idle=0; last=p; }
            if idle>=20 {
                println!("round {round} NO PROGRESS for 10s: inserts done {p}, tasks done {} of 32, write ch? entry_count {}", done.load(Ordering::SeqCst), c.entry_count());
                std::process::exit(0);
            }
        }
        for h in hs { h.await.unwrap(); }
        println!("round {round} finished");
    }
}
