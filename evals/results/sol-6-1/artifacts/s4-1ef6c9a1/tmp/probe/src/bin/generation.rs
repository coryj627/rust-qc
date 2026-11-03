use std::time::Duration;
fn main() {
    for count in [65_000, 66_000] {
        let c = moka::sync::Cache::builder().max_capacity(10).build();
        for n in 0..count {
            c.insert(0, n);
            if n % 32 == 0 { c.run_pending_tasks(); }
        }
        c.run_pending_tasks();
        c.invalidate_all();
        std::thread::sleep(Duration::from_millis(1));
        for _ in 0..10 { c.run_pending_tasks(); }
        println!("sync: {count} writes, get={:?}, entry_count={}, weight={}", c.get(&0), c.entry_count(), c.weighted_size());
    }
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        for count in [65_000, 66_000] {
            let c = moka::future::Cache::builder().max_capacity(10).build();
            for n in 0..count {
                c.insert(0, n).await;
                if n % 32 == 0 { c.run_pending_tasks().await; }
            }
            c.run_pending_tasks().await;
            c.invalidate_all();
            tokio::time::sleep(Duration::from_millis(1)).await;
            for _ in 0..10 { c.run_pending_tasks().await; }
            println!("future: {count} writes, get={:?}, entry_count={}, weight={}", c.get(&0).await, c.entry_count(), c.weighted_size());
        }
    });
}
