use moka::sync::Cache;
fn main() {
    let c: Cache<u32, u32> = Cache::new(100);
    for k in 0..3 { c.insert(k, k); }
    c.run_pending_tasks();
    for k in [0, 0, 1, 2, 0, 1] { assert!(c.get(&k).is_some()); c.run_pending_tasks(); }
    println!("no error");
}
