use moka::future::Cache;
use std::sync::Arc;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a1_upsert_lost_update() {
    let c: Cache<u32, u64> = Cache::new(100);
    let mut hs = vec![];
    for _ in 0..8 {
        let c = c.clone();
        hs.push(tokio::spawn(async move {
            for _ in 0..5000 {
                c.entry(1).and_upsert_with(|e| async move { e.map(|e| e.into_value()).unwrap_or(0) + 1 }).await;
            }
        }));
    }
    for h in hs { h.await.unwrap(); }
    let v = c.get(&1).await.unwrap();
    println!("a1: final = {v} (expected 40000)");
    assert_eq!(v, 40000);
}

#[test]
fn a1s_sync_control() {
    let c: moka::sync::Cache<u32, u64> = moka::sync::Cache::new(100);
    let mut hs = vec![];
    for _ in 0..8 {
        let c = c.clone();
        hs.push(std::thread::spawn(move || {
            for _ in 0..5000 {
                c.entry(1).and_upsert_with(|e| e.map(|e| e.into_value()).unwrap_or(0) + 1);
            }
        }));
    }
    for h in hs { h.join().unwrap(); }
    let v = c.get(&1).unwrap();
    println!("a1s: final = {v} (expected 40000)");
    assert_eq!(v, 40000);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a2_unit_error_collides() {
    let c: Cache<u32, u32> = Cache::new(10);
    let c2 = c.clone();
    let (tx, rx) = tokio::sync::oneshot::channel();
    let a = tokio::spawn(async move {
        c2.try_get_with(1, async move { tx.send(()).unwrap(); tokio::time::sleep(std::time::Duration::from_millis(200)).await; Err::<u32, ()>(()) }).await
    });
    rx.await.unwrap();
    let v = c.get_with(1, async { 42 }).await;
    println!("a2: {v} {}", a.await.unwrap().is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a3_predicate_skip() {
    let c: Cache<u32, u32> = Cache::builder().support_invalidation_closures().build();
    for i in 0..2000u32 { c.insert(i, i).await; }
    c.run_pending_tasks().await;
    c.invalidate_entries_if(|_, _| true).unwrap();
    for _ in 0..50 { c.run_pending_tasks().await; }
    let vis: Vec<_> = c.iter().map(|(k, _)| *k).collect();
    println!("a3: visible={vis:?}");
    assert!(vis.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a4_gen_wrap() {
    let c: Cache<u32, u32> = Cache::builder().time_to_live(std::time::Duration::from_millis(300)).build();
    for i in 0..70_000u32 { c.insert(1, i).await; }
    c.run_pending_tasks().await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    for _ in 0..5 { c.run_pending_tasks().await; }
    println!("a4: count={}", c.entry_count());
    assert_eq!(c.entry_count(), 0);
}

// Lost wakeup probe: many short-lived writers filling the write-op channel.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a5_write_hang_probe() {
    for round in 0..200 {
        let c: Cache<u32, u32> = Cache::new(100_000);
        let mut hs = vec![];
        for t in 0..1500u32 {
            let c = c.clone();
            hs.push(tokio::spawn(async move { c.insert(t, t).await; }));
        }
        let r = tokio::time::timeout(std::time::Duration::from_secs(20), async move { for h in hs { h.await.unwrap(); } }).await;
        assert!(r.is_ok(), "hang at round {round}");
    }
    let _ = Arc::new(());
}

// A6: cancelled maintenance task keeps the key lock while parked.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a6_cancelled_maintenance_blocks_same_key_insert() {
    use moka::notification::ListenerFuture;
    use futures_util::FutureExt;
    use std::time::Duration;
    let c: Cache<u32, u32> = Cache::builder()
        .time_to_live(Duration::from_millis(50))
        .async_eviction_listener(|_k, _v, _c| -> ListenerFuture {
            async { tokio::time::sleep(Duration::from_millis(200)).await; }.boxed()
        })
        .build();
    c.insert(1, 1).await;
    c.run_pending_tasks().await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    // Cancel maintenance while it is inside the listener.
    let r = tokio::time::timeout(Duration::from_millis(20), c.run_pending_tasks()).await;
    println!("a6: first run_pending_tasks timed out = {}", r.is_err());
    tokio::time::sleep(Duration::from_millis(500)).await;
    let r2 = tokio::time::timeout(Duration::from_secs(3), c.insert(1, 2)).await;
    println!("a6: insert(1) completed = {}", r2.is_ok());
    assert!(r2.is_ok(), "insert of the same key hung");
}

// A7: update over expired entry (async)
struct CreateOnly(std::time::Duration);
impl moka::Expiry<u32, u32> for CreateOnly {
    fn expire_after_create(&self, _: &u32, _: &u32, _: std::time::Instant) -> Option<std::time::Duration> { Some(self.0) }
}
#[tokio::test]
async fn a7_insert_over_expired_entry() {
    let c: Cache<u32, u32> = Cache::builder().expire_after(CreateOnly(std::time::Duration::from_millis(50))).build();
    c.insert(1, 1).await;
    c.run_pending_tasks().await;
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    c.insert(1, 2).await;
    let g = c.get(&1).await;
    println!("a7: {g:?}");
    assert_eq!(g, Some(2));
}

// A8: other tasks' timeouts make a healthy waiter panic.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a8_retry_exhaustion_from_cancellations() {
    use std::time::Duration;
    let c: Cache<u32, u32> = Cache::new(10);
    let mut hs = vec![];
    for i in 0..1200u64 {
        let c = c.clone();
        hs.push(tokio::spawn(async move {
            // Every caller is happy to wait a bounded time; init takes "forever".
            let _ = tokio::time::timeout(
                Duration::from_millis(5 + i),
                c.get_with(1, async { tokio::time::sleep(Duration::from_secs(60)).await; 0 }),
            ).await;
        }));
    }
    let mut panics = 0;
    let mut msg = String::new();
    for h in hs {
        if let Err(e) = h.await {
            panics += 1;
            if msg.is_empty() { msg = format!("{e:?}"); }
        }
    }
    println!("a8: tasks that panicked = {panics} {}", msg);
    assert_eq!(panics, 0);
}
