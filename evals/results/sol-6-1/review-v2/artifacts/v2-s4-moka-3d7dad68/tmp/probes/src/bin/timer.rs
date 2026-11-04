use moka::Expiry;
use std::{
    sync::{mpsc, Arc, Mutex},
    time::{Duration, Instant},
};
struct Exp;
impl Expiry<u32, u32> for Exp {
    fn expire_after_create(&self, k: &u32, _: &u32, _: Instant) -> Option<Duration> {
        (*k == 0).then_some(Duration::from_millis(1))
    }
    fn expire_after_read(
        &self,
        k: &u32,
        _: &u32,
        _: Instant,
        d: Option<Duration>,
        _: Instant,
    ) -> Option<Duration> {
        if *k == 1 {
            d.or(Some(Duration::from_millis(40)))
        } else {
            d
        }
    }
}
fn main() {
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let release_rx = Arc::new(Mutex::new(release_rx));
    let c = moka::sync::Cache::builder()
        .expire_after(Exp)
        .eviction_listener(move |k, _, _| {
            if *k == 0 {
                started_tx.send(()).unwrap();
                release_rx
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(5))
                    .unwrap();
            }
        })
        .build();
    c.insert(0, 0);
    c.insert(1, 1);
    c.insert(2, 2);
    c.run_pending_tasks();
    std::thread::sleep(Duration::from_millis(1200));
    let maintenance = std::thread::spawn({
        let c = c.clone();
        move || c.run_pending_tasks()
    });
    started_rx.recv_timeout(Duration::from_secs(3)).unwrap();
    let count = if std::env::args().any(|a| a == "--control") {
        0
    } else {
        1000
    };
    for _ in 0..count {
        assert_eq!(c.get(&2), Some(2));
    }
    assert_eq!(c.get(&1), Some(1));
    release_tx.send(()).unwrap();
    maintenance.join().unwrap();
    std::thread::sleep(Duration::from_millis(1300));
    for _ in 0..5 {
        c.run_pending_tasks();
    }
    assert_eq!(c.get(&1), None);
    let maintained_count = c.entry_count();
    let visible_count = c.iter().count();
    let retained = c.remove(&1);
    println!("visible entries: {visible_count}; maintained entry_count: {maintained_count}; removed retained value: {retained:?}");
    assert_eq!(
        maintained_count, 1,
        "expired entry should have been removed by maintenance"
    );
}
