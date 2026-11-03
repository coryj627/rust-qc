use moka::sync::Cache;
fn main() {
    let c: Cache<u32, u32> = Cache::new(10);
    c.insert(1, 1);
    let e = c.entry(1).or_insert_with_if(|| 2, |_| true);
    println!("or_insert_with_if replacing: value={} fresh={} old_replaced={} (cached now {:?})", e.value(), e.is_fresh(), e.is_old_value_replaced(), c.get(&1));
    let e = c.entry(1).and_upsert_with(|_| 3);
    println!("and_upsert_with replacing: fresh={} old_replaced={}", e.is_fresh(), e.is_old_value_replaced());
}
