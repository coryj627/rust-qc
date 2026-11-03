//! moka-4: Deque::move_to_back keeps a pointer derived from a temporary &mut as the
//! new tail. Reaches it only through sync::Cache get/insert/run_pending_tasks.
//!
//! Access order after the inserts: 0 1 2 3.
//! get(0): move_to_back(0). Node 0 is the tail; the new tail pointer and node 3's
//!         `next` both come from the temporary &mut inside move_to_back.
//! get(0): the entry's own stored node pointer reads node 0 (`deq.contains`), which
//!         is a foreign read for the pointer kept in the deque. It is frozen.
//! get(3): move_to_back(3) writes node 0's `prev` through the frozen pointer.
use moka::{policy::EvictionPolicy, sync::Cache};

fn main() {
    let cache: Cache<u32, u32> = Cache::builder()
        .max_capacity(16)
        .eviction_policy(EvictionPolicy::lru())
        .build();
    for k in 0..4 {
        cache.insert(k, k);
    }
    cache.run_pending_tasks();
    for k in [0u32, 0, 3] {
        let _ = cache.get(&k);
        cache.run_pending_tasks();
        println!("get({k}) replayed by housekeeping");
    }
    println!("finished with no Miri error, entry_count={}", cache.entry_count());
}
