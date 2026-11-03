// invalidate_entries_if: boundary skip and second predicate
use moka::sync::Cache;
fn main() {
    // A. one predicate, > batch size entries
    let c: Cache<u32, u32> = Cache::builder().support_invalidation_closures().build();
    for i in 0..2000 { c.insert(i, i); }
    c.run_pending_tasks();
    c.invalidate_entries_if(|_, _| true).unwrap();
    for _ in 0..20 { c.run_pending_tasks(); }
    let left: Vec<u32> = (0..2000).filter(|k| c.contains_key(k)).collect();
    println!("A one predicate over 2000 entries: entry_count={} survivors={:?}", c.entry_count(), left);

    // B. second predicate registered while first one is in progress
    let c: Cache<u32, u32> = Cache::builder().support_invalidation_closures().build();
    for i in 0..1200 { c.insert(i, i); }
    c.run_pending_tasks();
    c.invalidate_entries_if(|k, _| *k == u32::MAX).unwrap(); // matches nothing
    c.run_pending_tasks();
    c.invalidate_entries_if(|_, _| true).unwrap(); // should remove everything
    for _ in 0..20 { c.run_pending_tasks(); }
    let left: Vec<u32> = (0..1200).filter(|k| c.contains_key(k)).collect();
    println!("B second predicate: entry_count={} survivors={} first={:?}", c.entry_count(), left.len(), &left[..left.len().min(10)]);

    // C. second predicate registered after one scan with few entries
    let c: Cache<u32, u32> = Cache::builder().support_invalidation_closures().build();
    for i in 0..600 { c.insert(i, i); }
    c.run_pending_tasks();
    c.invalidate_entries_if(|k, _| *k == u32::MAX).unwrap();
    c.run_pending_tasks();
    c.invalidate_entries_if(|_, _| true).unwrap();
    for _ in 0..20 { c.run_pending_tasks(); }
    let left: Vec<u32> = (0..600).filter(|k| c.contains_key(k)).collect();
    println!("C: entry_count={} survivors={} first={:?}", c.entry_count(), left.len(), &left[..left.len().min(10)]);
}
