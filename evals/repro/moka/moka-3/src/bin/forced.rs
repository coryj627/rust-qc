//! moka-3: a timer node popped after a user thread cleared its expiration is freed,
//! but DeqNodes.timer_node still points at it. Safe code only.
//!
//! The eviction listener runs on the housekeeping thread between two timer wheel
//! pops, so it can hold housekeeping at that exact point while a second thread does
//! an ordinary insert. No sleep-based timing is used for the interleaving itself.
use moka::{notification::RemovalCause, sync::Cache, Expiry};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc, Mutex,
};
use std::time::{Duration, Instant};

const A: u32 = 1;
const B: u32 = 2;

// Miri's virtual clock runs fast (each basic block costs microseconds), so a level 0
// bucket (1.074 s) is too narrow there. Under Miri the same shape uses a level 1 bucket
// (68.7 s), and sleeping through it costs nothing on the virtual clock.
//   (A expires, B expires, wake), measured from cache creation.
const TIMES_MS: (u64, u64, u64) = if cfg!(miri) { (150_000, 195_000, 155_000) } else { (1_100, 1_900, 1_300) };

struct E {
    created: Instant,
}
impl Expiry<u32, u32> for E {
    fn expire_after_create(&self, k: &u32, _: &u32, at: Instant) -> Option<Duration> {
        // Both land in one timer wheel bucket, so housekeeping pops A then B in one walk.
        let ms = if *k == A { TIMES_MS.0 } else { TIMES_MS.1 };
        Some((self.created + Duration::from_millis(ms)).saturating_duration_since(at))
    }
    fn expire_after_update(&self, _: &u32, _: &u32, _: Instant, left: Option<Duration>) -> Option<Duration> {
        println!("expire_after_update: duration_until_expiry={left:?}, returning None");
        None // Documented: clears the expiration time.
    }
}

fn round(n: u32) {
    let (go_tx, go_rx) = mpsc::channel::<()>();
    let (ack_tx, ack_rx) = mpsc::channel::<()>();
    let ack_rx = Mutex::new(ack_rx);
    let go_tx = Mutex::new(go_tx);
    let hooked = Arc::new(AtomicBool::new(false));
    let hooked2 = Arc::clone(&hooked);

    // The wheel's origin is set while building, so anchor the times just before it.
    let created = Instant::now();
    let cache: Cache<u32, u32> = Cache::builder()
        .expire_after(E { created })
        .eviction_listener(move |k: Arc<u32>, _v, cause| {
            if *k == A && cause == RemovalCause::Expired && !hooked2.swap(true, Ordering::SeqCst) {
                // Housekeeping is inside the timer wheel iterator. A was just popped;
                // B is still queued behind it. Let the other thread update B now.
                go_tx.lock().unwrap().send(()).unwrap();
                ack_rx.lock().unwrap().recv().unwrap();
            }
        })
        .build();
    println!("round {n}: build took {:?} on this clock", created.elapsed());

    let c2 = cache.clone();
    let updater = std::thread::spawn(move || {
        if go_rx.recv().is_ok() {
            c2.insert(B, 200); // expire_after_update -> None clears B's expiration.
            ack_tx.send(()).unwrap();
        }
    });

    cache.insert(A, 100);
    cache.insert(B, 100);
    cache.run_pending_tasks(); // Both get timer nodes.
    println!("round {n}: scheduled at {:?}", created.elapsed());

    // A has expired and B has not, so B's update is a real update.
    std::thread::sleep((created + Duration::from_millis(TIMES_MS.2)).saturating_duration_since(Instant::now()));
    cache.run_pending_tasks(); // Pops A, listener runs, then pops and frees B's node.
    println!("round {n}: listener hook fired = {} at {:?}", hooked.load(Ordering::SeqCst), created.elapsed());
    cache.run_pending_tasks(); // Applies B's queued Upsert: deschedule(stale timer_node).
    println!("round {n}: get(B) = {:?}", cache.get(&B));
    drop(cache);
    drop(updater);
}

fn main() {
    for n in 0..3 {
        round(n);
    }
    println!("finished: no fault seen");
}
