use std::time::Duration;
fn main() {
    for updates in [100, 65534, 65535, 65536] {
        let c = moka::sync::Cache::builder().max_capacity(1).time_to_live(Duration::from_millis(100)).build();
        c.insert(0u32, 0u32);
        c.run_pending_tasks();
        for i in 0..updates { c.insert(0, i); c.run_pending_tasks(); }
        std::thread::sleep(Duration::from_millis(150));
        for _ in 0..3 { c.run_pending_tasks(); }
        println!("updates={updates}, get={:?}, count={}, weight={}", c.get(&0), c.entry_count(), c.weighted_size());
    }
}
