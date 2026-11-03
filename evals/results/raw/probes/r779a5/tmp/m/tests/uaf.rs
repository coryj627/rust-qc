use moka::{sync::Cache, Expiry};
use std::{sync::{mpsc, Mutex}, time::{Duration, Instant}};

struct Ex;
impl Expiry<u32, u32> for Ex {
    fn expire_after_create(&self, k: &u32, _: &u32, _: Instant) -> Option<Duration> {
        if cfg!(miri) {
            // Miri's virtual clock jumps over sleeps but charges for work, so use the
            // much wider level-1 buckets: key 1 is in level 0, key 2 in level 1 index 1.
            Some(if *k == 1 { Duration::from_secs(69) } else { Duration::from_secs(130) })
        } else {
            Some(if *k == 1 { Duration::from_millis(100) } else { Duration::from_millis(1500) })
        }
    }
    fn expire_after_read(&self, k: &u32, _: &u32, _: Instant, d: Option<Duration>, _: Instant) -> Option<Duration> {
        if *k == 2 { None } else { d }
    }
}

#[test]
fn timer_node_dangles_when_expiry_cleared_during_advance() {
    let (req_tx, req_rx) = mpsc::channel::<()>();
    let (done_tx, done_rx) = mpsc::channel::<()>();
    let done_rx = Mutex::new(done_rx);
    let req_tx = Mutex::new(req_tx);
    let cache: Cache<u32, u32> = Cache::builder()
        .expire_after(Ex)
        .eviction_listener(move |k, _, _| {
            if *k == 1 {
                // Hand over to another thread, which does an ordinary get(&2) while
                // this thread is inside the timer-wheel advance.
                req_tx.lock().unwrap().send(()).unwrap();
                done_rx.lock().unwrap().recv().unwrap();
            }
        })
        .build();
    let c2 = cache.clone();
    let reader = std::thread::spawn(move || {
        req_rx.recv().unwrap();
        let v = c2.get(&2);
        println!("reader: get(2) = {v:?}");
        done_tx.send(()).unwrap();
    });
    cache.insert(1, 1);
    cache.insert(2, 2);
    cache.run_pending_tasks();
    std::thread::sleep(if cfg!(miri) { Duration::from_secs(70) } else { Duration::from_millis(1200) });
    cache.run_pending_tasks(); // pops 1 (expired, reader clears 2's expiry), then drops 2's node
    println!("after advance: count={}", cache.entry_count());
    reader.join().unwrap();
    cache.run_pending_tasks(); // applies the queued ReadOp: deschedule(dangling)
    println!("after applying read op: get(2)={:?}", cache.get(&2));
    cache.invalidate(&2);
    cache.run_pending_tasks();
    println!("end: count={}", cache.entry_count());
}
