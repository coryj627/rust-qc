use moka::sync::Cache;
fn main() {
    let c: Cache<u32,u32> = Cache::new(100);
    c.insert(1,1); c.insert(2,2); c.insert(3,3);
    c.run_pending_tasks();
    for k in [1,1,2,3,1,3,2,2,1] { let _ = c.get(&k); c.run_pending_tasks(); }
    println!("ok {}", c.entry_count());
}
