# Audit report: moka 0.12.11 (`crate/src/`)

## Summary

The crate builds, and its own tests pass. Under concurrency, though, its policy bookkeeping can be corrupted. Normal multi-threaded use can leave a bounded cache holding almost nothing or growing without limit. Invalidated values can come back, and async `and_upsert_with` loses updates.

There are 22 findings:  Every finding comes from a probe through the public API unless it says "read only". Probes are in `probe/src/bin/` in the workspace, and `crate/` was not changed. Nothing in the repository or its output addressed me.

## High

**1. Remove and upsert ops processed out of order leave permanent phantom entries**
- **Location:** `src/sync/base_cache.rs:1469` (`handle_upsert`), `src/future/base_cache.rs:1620`. Stuck eviction: `src/sync/base_cache.rs:2278` and `skip_updated_entry_ao`.
- **Problem:** Thread A updates the map but has not yet sent its `Upsert`. Thread B then invalidates the key and its `Remove` is processed first. The late `Upsert` sees `is_admitted == false` and admits an entry that is no longer in the map.
  - That node is never removed, because `remove_if` returns `None` and the node is only skipped.
  - If the key is live again, `skip_updated_entry_ao` moves the live entry's node instead, so the phantom stays at the front and blocks eviction.
- **User sees:** 4 threads ran `insert`/`invalidate` on 4 keys for about 6 s (`p5`, `p10`):
  - TinyLFU, capacity 1000: `entry_count=965` with 0 real entries. Only 3 of 500 fresh keys were then retained, against 500 in a control cache.
  - LRU, capacity 1000: `entry_count=40461` with 0 real entries.
  - LRU, capacity 1000, hot keys left live, then 40000 fresh inserts: 40004 real entries held, so capacity is not enforced at all.
  - `future::Cache`: `entry_count=952` with 0 real entries.

**2. Concurrent first insert of a key permanently inflates `weighted_size`**
- **Location:** `src/sync/base_cache.rs:1458-1461`, `src/future/base_cache.rs:1609`.
- **Problem:** If thread B's update op runs before thread A's original insert op, B's op admits the entry with its full weight. A's op (`old_weight=0`) then counts as an update and adds the weight again. Removal subtracts only one weight, so the excess never goes away.
- **User sees:** 4 threads inserted each key at the same moment, with no removes and all weights 1 (`p7`, `p8`):
  - Unbounded capacity: 300000 entries, `weighted_size=318559`.
  - LRU, capacity 5000: `weighted=12264`, 0 real entries, so the cache evicts everything.
  - TinyLFU, capacity 5000: 4706 entries, `weighted=5000`.

**3. Async compute methods break their serialization guarantee and lose updates**
- **Location:** `src/future/value_initializer.rs:350`; the same pattern is in `try_compute_if_nobody_else`.
- **Problem:** `set_waiter_value(ReadyNone)` removes the waiter and releases its lock before `Op::Put` or `Op::Remove` is applied. The next caller then reads the old value. The sync version releases only after the write.
- **User sees:** The docs say "calls on the same key are executed serially". 8 tasks made 20000 increments each (`p2`):
  - `and_upsert_with` gave 159942 instead of 160000.
  - `and_compute_with` gave 159904 instead of 160000.
  - The sync cache gave the correct 160000.

**4. Aliasing undefined behavior in `Deque::move_to_back`, reachable from safe code**
- **Location:** `src/common/deque.rs:196-219`.
- **Problem:** `NonNull::from(node)` is derived from a local `&mut` and stored in the list, while the original raw pointer in `DeqNodes` stays in use (`node.as_ref()` in `move_to_back_ao`). That read freezes the stored tag, and a later write through it is forbidden.
- **User sees:** Nothing observed at runtime, and I saw no miscompilation. `Cache::new(100)`, 3 inserts, then `get` plus `run_pending_tasks` is enough. Miri with `-Zmiri-tree-borrows` reports "write access ... is forbidden" at `deque.rs:215` (`m3`).
  - Tree Borrows is still an experimental model, so this rests on that one tool.
  - Miri's Stacked Borrows mode stops earlier, inside `crossbeam-epoch`, so it could not check this path.
  - The 18 `common::` unit tests pass under Miri's default mode because they do not hit this sequence.
  - Miri stops at the first error, so anything later in the run went unchecked.

## Medium

**5. `invalidate_entries_if` skips one entry per batch, and those entries come back**
- **Location:** `src/sync/base_cache.rs:2153,2170`, `src/future/base_cache.rs:2351,2368`.
- **Problem:** `iter.peek()` advances the deque's own cursor, and the peeked element is lost when the `Peekable` is dropped. The entry after each 384-entry batch is never scanned. Once the predicate retires, nothing filters it any more.
- **User sees:** With 2000 entries and `invalidate_entries_if(|_, _| true)`, keys 384, 769, 1154, 1539 and 1924 are readable again (`p1`). The docs say these methods "must not return invalidated values".

**6. A predicate registered during a scan can miss nearly everything**
- **Location:** `src/sync/invalidator.rs:175,199`, `src/sync/base_cache.rs:2184`, and `src/future/invalidator.rs:175`, `src/future/base_cache.rs:2383`.
- **Problem:** When predicate A retires part-way through a scan, `is_done` is false because B is registered, so the cursor is not reset. B starts mid-queue and retires at the end without ever seeing earlier entries.
- **User sees:** With 1000 old entries, A registered, new writes arriving, then B matching all old keys, 1000 of 1000 stayed readable. The control with B alone left 2 (finding 5).

**7. After 65536 updates to one key, the entry is permanently dirty**
- **Location:** `src/common/concurrent/entry_info.rs:97`.
- **Problem:** When `entry_gen` wraps to 0, `current >= value` stays true with `policy_gen` at 65535, so `policy_gen` stops moving. Every expiry, eviction and invalidation path then skips the entry as "being updated".
- **User sees (`p1`):**
  - TTL: after 70000 updates and the TTL passing, `entry_count=1` with `get=None`. The entry is never removed and no listener fires. The control at 60000 updates reaches 0.
  - LRU, capacity 100: the key survives 2000 newer keys.

**8. A blind re-insert over a per-entry-expired key is lost**
- **Location:** `src/sync/base_cache.rs:689-696`, `src/future/base_cache.rs:821-826`.
- **Problem:** For an expired entry, `checked_duration_since` gives `None`. The default `expire_after_update` returns that `None`, so `duration == current_duration` and the old expiration time in the past is kept.
- **User sees:** With an `Expiry` that implements only `expire_after_create`: insert, wait past expiry, `insert(1, 20)`, and `get` returns `None`, in both sync and async. `and_upsert_with` returns `is_fresh=true`, yet the value is unreadable (`p3`).

**9. `remove` and compute `Op::Remove` return expired or invalidated values**
- **Location:** `src/sync/cache.rs:1601`, `src/future/cache.rs:1959` (async path read only).
- **Problem:** The map entry is removed and returned with no validity check.
- **User sees (`p4`):**
  - `get` gives `None` but `remove` gives `Some(old)` after TTL, after `invalidate_all`, and after a predicate.
  - `and_compute_with` sees `None`, returns `Op::Remove`, and gets `Removed(4)` rather than `StillNone`.

**10. A panic while the waiter is installed poisons or hangs that key forever**
- **Location:** `src/sync/value_initializer.rs:137,153` (`try_init_or_read`), `:244,278` and the loop at `:202-222` (`try_compute`).
- **Problem:** Only `init` and `f` are inside `catch_unwind`. A panic in the weigher, `Expiry` or `V::clone` leaves the waiter in the map.
- **User sees (`p1`, `p6`):**
  - One panicking weigher or `V::clone` makes later `get_with` calls on that key panic every time with "Got unexpected state `Computing`... might be a bug in Moka".
  - After the same panic in `and_upsert_with`, the next call on that key spins forever. My watchdog gave up after 5 s.

**11. `entry_by_ref().or_try_insert_with()` returns an entry with no key**
- **Location:** `src/sync/cache.rs:1422` (hard-coded `false` instead of `need_key`).
- **User sees:** On an existing key, `entry.key()` panics with "Bug: Key is None", and `{:?}` on the entry would hit the same `key()` call (`p2`, `key()` only). The async version passes `need_key` correctly.

**12. `get_with` panics when it shares a waiter with `try_get_with::<_, ()>`**
- **Location:** `src/sync/value_initializer.rs:348`, `src/future/value_initializer.rs:592`, `src/sync/cache.rs:1053`, `src/future/cache.rs:1597`.
- **Problem:** `get_with` uses `TypeId::of::<()>()` as its waiter key, which collides with `E = ()`.
- **User sees:** While `try_get_with` is returning `Err(())`, a concurrent `get_with` on that key panics with "entered unreachable code", in both sync and async (`p1`, `p6`).

**13. One panic during async maintenance kills the housekeeper permanently**
- **Location:** `src/future/housekeeper.rs:173`.
- **Problem:** The poisoned `Shared` future stays in `current_task` and is awaited again on every run.
- **User sees:** A predicate that panicked once made every later `run_pending_tasks` panic with "inner future panicked during poll", and a task doing inserts afterwards panicked too (`p2`). The sync cache recovers (`p9`).

**14. The key lock is not exclusive, so listener calls for one key overlap**
- **Location:** `src/sync/key_lock.rs:32` (`src/future/key_lock.rs:32` has the same code, read only).
- **Problem:** `Drop` can remove a lock from the map just as another thread clones it. The next arrival then creates a second lock for the same key. A test comment in `src/sync/cache.rs:4687` states the rule: "Notifications for the same key must not overlap."
- **User sees:** 12 overlapping listener calls on the same key in 6 s, insert-only, 4 threads, 8 keys (`p10`). This matters to listeners such as the doc example that manages data files.

## Low

**15. `is_old_value_replaced()` is false when `or_insert_with_if` replaces a value**
- **Location:** `src/sync/cache.rs:1050`, `src/future/cache.rs:1594`.
- **User sees:** `value=2, fresh=true, old_replaced=false` (`p3`). The method's doc says it returns true when an old value was replaced.

**16. `RemovalCause` ignores per-entry expiry**
- **Location:** `src/sync/base_cache.rs:2306,2336`, `src/future/base_cache.rs:2513,2554`.
- **User sees:** With an `Expiry`-based cache, overwriting an expired entry reports `Replaced` and invalidating one reports `Explicit`, rather than `Expired` (`p9`, sync only).

**17. `SegmentedCache` can hold more than its `max_capacity`**
- **Location:** `src/sync/segment.rs:723`.
- **User sees:** `SegmentedCache::new(8, 16)` reports `policy.max=8` but held 16 entries (`p3`). Reading the same code, an entry heavier than `max/segments` is rejected even when it would fit the total.

**18. The sketch size estimate for weighted caches is inverted (read only)**
- **Location:** `src/sync/base_cache.rs:1348`, `src/future/base_cache.rs:1488`.
- **Problem:** The formula multiplies `entry_count` by `weighted/max`. A half-full cache with 500 entries gets a size of 250 instead of about 1000, and the sketch is never resized. The likely result is a worse hit rate.

**19. A panic in `expire_after_create` leaves an entry the policy never tracks**
- **Location:** `src/sync/base_cache.rs:551-560`, `src/future/base_cache.rs:552`.
- **User sees:** The entry stays readable and is never evicted. With capacity 10: `get=Some(666)`, `entry_count=10`, `iter=11` (`p4`, sync only).

**20. Nested `get_with` on the same key deadlocks, and this is undocumented**
- **Location:** `src/sync/value_initializer.rs:110`, `src/future/value_initializer.rs:189`.
- **User sees:** The nested sync call hung; my watchdog gave up after 3 s (`p4`). The async path was read only.

**21. Possible lost wakeup when the async write channel is full (read only)**
- **Location:** `src/future/base_cache.rs:646-679`.
- **Problem:** `listen()` is called after `try_send` fails, with no re-check. A notify in that gap is missed, and the task then waits for later cache activity. Stress runs (25 rounds, 32 tasks) did not reproduce a stall.

**22. Hygiene**
- **rustdoc fails with `-D warnings`:** `src/sync/entry_selector.rs:201,743` link to a missing target (`try_append_value_async.rs`) in sync docs, and `src/common/concurrent/arc.rs:83,99` has a bad link and a bare URL.
- **SAFETY comments:** 76 unsafe blocks and 13 `unsafe impl`s have none. The `Send`/`Sync` impls I read cannot be misused, because every constructor requires `K, V: Send + Sync`.
- **Other lint counts:**
  - 29 asserts without messages
  - 20 functions missing `# Errors`, 1 missing `# Panics`
  - 8 public types without `Debug`, including `Iter`
  - 9 missing doc items
- **Unwraps:** 13 flagged, and those I read in production code are guarded by invariants.
