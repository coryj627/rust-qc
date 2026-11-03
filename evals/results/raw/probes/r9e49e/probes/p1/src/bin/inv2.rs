use moka::sync::Cache;
fn main() {
    let c: Cache<u32, u32> = Cache::builder().support_invalidation_closures().build();
    for i in 0..1000 { c.insert(i, i); }
    c.run_pending_tasks();
    std::thread::sleep(std::time::Duration::from_millis(2));
    c.invalidate_entries_if(|k, _| *k == u32::MAX).unwrap(); // A: matches nothing
    std::thread::sleep(std::time::Duration::from_millis(2));
    for i in 1000..3000 { c.insert(i, i); }
    c.run_pending_tasks();
    std::thread::sleep(std::time::Duration::from_millis(2));
    c.invalidate_entries_if(|_, _| true).unwrap(); // B: everything so far
    for _ in 0..30 { c.run_pending_tasks(); }
    let left = (0..3000).filter(|k| c.contains_key(k)).count();
    println!("A finished part way, then B: survivors of 3000 = {left}, entry_count={}", c.entry_count());
}
