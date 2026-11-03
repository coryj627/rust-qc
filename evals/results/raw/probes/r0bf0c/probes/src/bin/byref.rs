use moka::sync::Cache;
fn main() {
    let cache: Cache<String, u32> = Cache::new(10);
    cache.insert("a".to_string(), 1);
    let e = cache.entry_by_ref("a").or_try_insert_with(|| Ok::<u32, ()>(2)).unwrap();
    println!("fresh={} key={:?}", e.is_fresh(), e.key());
}
