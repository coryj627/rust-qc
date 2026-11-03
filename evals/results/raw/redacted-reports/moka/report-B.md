# Audit of moka 0.12.11: 20 findings

`crate/` was not changed. One probe patched a copy at `tmp/c2`. All probes are harnesses under `tmp/h` and `tmp/m` in the workspace, run through `./run`. The baseline is clean: the lib suite passes 150 with 5 ignored, the integration tests pass, and all feature combinations compile. No text in the repository addressed me.

Paths are relative to `crate/src/`. `base_cache.rs` line numbers are for `sync/`, with the `future/` counterpart given where it differs.

## High severity

**1. Use-after-free of a timer wheel node, reachable from safe code**
- **Where:** `common/timer_wheel.rs:494-526`. The `if let Some(t) = expiration_time` has no `else`.
- **What is wrong:** if a popped node's entry has had its expiration cleared, the `Box` is dropped silently and `DeqNodes.timer_node` keeps the dangling pointer. The expiration is cleared when `Expiry::expire_after_read` or `expire_after_update` returns `None`, and the timer update is not applied first. The comment at `sync/base_cache.rs:342` says this case is fine. The later `deschedule` or `reschedule` touches freed memory (`base_cache.rs:1755`, `1768`).
- **What a caller sees:** memory corruption or panics in `run_pending_tasks`.
- **How established:**
  - `tmp/m/tests/uaf2.rs` is single-threaded with no listener. It uses `or_insert_with_if`, whose second `replace_if` call makes the internal unrecorded read hit the entry. Natively it panics with `unreachable` at `timer_wheel.rs:283`.
  - Miri reports "dangling reference (use-after-free)" at `timer_wheel.rs:276` via `deschedule` and `base_cache.rs:1768`.
  - `tmp/m/tests/uaf.rs` uses a plain `get` from another thread during maintenance. Natively it panics with "index out of bounds: the len is 5 but the index is 250" at `timer_wheel.rs:279`, and Miri reports UB via `apply_reads` and `base_cache.rs:1755`.
- The listener in `uaf.rs` only pins the timing. Both caches share this iterator. I ran the probes on the sync cache only.

**2. An `Upsert` op applied after its `Remove` op admits a phantom entry permanently**
- **Where:** `sync/base_cache.rs:1458-1475` (`handle_upsert`, future `:1609-1620`). The map insert happens before the op is sent (`sync/cache.rs:1487-1489`). Nothing checks that the entry is still current before admitting it.
- **What is wrong:** an `insert` racing an `invalidate` leaves a counted entry that is not in the map. Nothing can ever remove it.
- **How established:**
  - `t10b`, no callbacks, 4 threads on 16 keys: `entry_count=29143 weighted=30076 iter=0`.
  - `t10d`, capacity 1000: after the stress `weighted=1000 live=0`, and 500 fresh inserts into the logically empty cache left `live=0`. The cache stops caching.
  - `t10c`, LRU with `max_capacity=10`, with the key re-inserted live: `entry_count=4902`, so the bound is gone. `evict_lru_entries` keeps retrying the phantom at the deque head, because `skip_updated_entry_ao` (`base_cache.rs:2019-2025`) moves the live entry's node instead (reasoned from the code; the probe shows only the count).
- All probes ran on the sync cache. The future `handle_upsert` has the same structure (read only).

**3. Inserting over an expired, not-yet-evicted entry makes the new value invisible**
- **Where:** `sync/base_cache.rs:689-705` (future `:821-826`).
- **What is wrong:** the old entry's expiration is in the past, so `checked_duration_since` gives `None`. The default `expire_after_update` returns that `None`, `duration == current_duration`, and the stale expiration is kept. `expire_after_create` is not called because the map path is `on_modify`.
- **What a caller sees:** with an `Expiry` overriding only `expire_after_create`, `insert(k, 2); get(&k)` returns `None`. `get_with` recomputes and then `get` is still `None`, so the key behaves as a permanent miss until eviction.
- **How established:** `t11`, `t11c` and async `a7` all fail. Control `t11b` (`invalidate`, then `insert`) returns `Some(2)`.

**4. `invalidate_entries_if` skips every 385th entry**
- **Where:** `sync/base_cache.rs:2153`, `2170` (future `:2351`, `2368`).
- **What is wrong:** `iter.peek()` on the `Peekable` advances the deque cursor, and the peeked element is thrown away when the `Peekable` drops. It is never tested. Once the predicate is removed, the entry is visible again.
- **How established:** `t2` and async `a3`, 2000 entries with predicate always true. Survivors are keys 384, 769, 1154, 1539, 1924, still returned by `iter()`.

**5. A predicate registered while another scan is in progress misses entries**
- **Where:** `sync/invalidator.rs:175` (snapshot taken only when empty) and `sync/base_cache.rs:2184-2186` (cursor reset only when the registry is empty).
- **What is wrong:** the next predicate starts from the old cursor position.
- **How established:**
  - `t9d`: all 1150 target entries survive P2, and `get(&0)=Some(0)` after P2 is gone.
  - `t9b`: head entry 0 survives, because the end-of-queue `peek` wraps the cursor to the head's successor.
- `future/invalidator.rs:175` and `future/base_cache.rs:2384` have the same code (read only).

**6. `entry_gen` wraparound leaves an entry dirty for up to 65,535 further updates**
- **Where:** `common/concurrent/entry_info.rs:97`. `current >= value` is true for `current=65535, value=0`, so `policy_gen` stays at 65535 and `is_dirty()` stays true until `entry_gen` reaches 65535 again.
- **What a caller sees:** a dirty entry is skipped by TTL, TTI, timer and size eviction, so it can sit in memory indefinitely once updates stop. A key that has been updated about 65.5k times can no longer be evicted.
- **How established:**
  - `t1`: 70,000 updates with a TTL gives `entry_count=1` forever after expiry, while `get` returns `None`.
  - Control `t1b`: 60,000 updates gives `entry_count=0`.
  - `t1c`: `max_capacity=1` with LRU, key 1 stays cached after 8 later inserts of other keys.
  - Async `a4` also fails.

**7. Async `and_compute_with` / `and_upsert_with` lose updates**
- **Where:** `future/value_initializer.rs:350` and `:488`.
- **What is wrong:** `set_waiter_value(ReadyNone)` releases the lock and removes the waiter before the op is applied at `:373-395`. The docs at `future/entry_selector.rs:163-165` guarantee serial execution.
- **How established:** `a1`, 8 tasks × 5000 increments on a 4-worker runtime: final value 39944, expected 40000. The sync control `a1s` gets 40000. It removes the waiter only after the op. Reasoned from the code, the same early release also defeats `and_try_compute_if_nobody_else`.

## Medium severity

**8. `weighted_size` drifts under racing inserts**
- **Where:** `sync/base_cache.rs:525`, `650` (future `:525`, `780`).
- **What is wrong:** `set_policy_weight` runs inside `on_modify`, which is retried on CAS failure, before the update is published. The next attempt reads the wrong `old_weight`. Reordered ops (finding 2) add to this.
- **What a caller sees:** size eviction works from a wrong total.
- **How established:** `t12`, inserts only: `weighted_size=18718`, true weight `4524`, 8 entries.

**9. Cancelled async maintenance can hang a same-key write**
- **Where:** `future/housekeeper.rs:172-173` parks the cancelled task. The key lock is taken in `evict_expired_entries_using_timers` and the other eviction paths and held across the listener await.
- **What is wrong:** `insert` waits on the key lock without driving maintenance.
- **How established:** `a6`, with `timeout(run_pending_tasks)` cancelled during a slow async listener. `insert(1, 2)` then does not complete within 3 s, well after the listener's own 200 ms sleep would have ended. Other cache activity would resume the task (read only).

**10. Sync `entry_by_ref(..).or_try_insert_with` panics on a cache hit**
- **Where:** `sync/cache.rs:1422` passes `false` instead of `need_key`.
- **What a caller sees:** `Entry::key()` and `Debug` panic with "Bug: Key is None".
- **How established:** `t3` fails, owned-key control `t3b` passes. The future version is correct.

**11. `get_with` panics when racing `try_get_with::<_, ()>` on the same key**
- **Where:** `sync/value_initializer.rs:345-349` and `future/value_initializer.rs:589-593`. `get_with` uses `TypeId::of::<()>()`, which collides with `E = ()`. The panics are at `sync/cache.rs:1053` and `future/cache.rs:1597`.
- **What a caller sees:** `get_with` receives the other caller's `Err` and hits `unreachable!`.
- **How established:** `t4` and `a2` both panic.

**12. A panic outside `catch_unwind` in sync `get_with` poisons the key permanently**
- **Where:** `sync/value_initializer.rs:137` and `:153`; `try_compute` at `:229`, `:278`, `:289`.
- **What is wrong:** a panic there unwinds with the waiter still in the map in state `Computing`. Likely sources are the weigher, `Expiry` or `V::clone`. The future version has a guard.
- **How established:** `t5`, weigher panics once. Every later `get_with` panics with "Got unexpected state `Computing`" at `:127`. The `try_compute` lines were read only.

**13. `remove` and compute return values that reads treat as absent**
- **Where:** `sync/cache.rs:1601-1641`, `sync/value_initializer.rs:288-296`, `future/cache.rs` `invalidate_with_hash`. Removal has no validity check.
- **How established:**
  - `t8`: after `invalidate_all`, `get=None` but `remove=Some(1)`.
  - `t8b`: the same after TTL expiry.
  - `t8c`: the compute closure is given `None`, returns `Op::Remove`, and the result is `Removed(old value)`.

## Low severity

**14. Wrong `RemovalCause` for entries expired by `Expiry`**
- **Where:** `sync/base_cache.rs:2306-2355` (future `:2513`, `:2554`). Only TTL and TTI are checked.
- **How established:** `t7` reports `Replaced` and `Explicit`. TTL control `t7b` reports `Expired` for both.

**15. `is_old_value_replaced()` is false when `or_insert_with_if` replaces a value**
- **Where:** `sync/cache.rs:1050`, `future/cache.rs:1594`.
- **How established:** `t13` prints `value=2 fresh=true replaced=false`. Sync only; the future line was read.

**16. `SegmentedCache` exceeds `max_capacity`**
- **Where:** `sync/segment.rs:723-724` rounds each segment's capacity up.
- **What is wrong:** the docs say it stores "up to the `max_capacity`".
- **How established:** `t14`, capacity 10 with 4 segments: `entry_count=12`.

**17. Aliasing violation in `Deque::move_to_back`**
- **Where:** `common/deque.rs:196`, `:210-215`. The tail pointer is stored from a `&mut`-derived `NonNull`.
- **How established:**
  - Miri with Tree Borrows reports "write access… forbidden" at `deque.rs:215` during ordinary repeated `get`s (`tmp/m/tests/sb.rs`).
  - In a patched copy (`tmp/c2`) using raw pointers throughout, the same test passes.
  - The Tree Borrows model is experimental.
  - Stacked Borrows stopped earlier on an error inside `crossbeam-epoch`, a dependency, so it gives no signal here.

**18. Frequency sketch sizing formula looks inverted**
- **Where:** `sync/base_cache.rs:1348`, `future/base_cache.rs:1488`.
- **What is wrong:** it computes `entry_count * (weighted/max)`. The estimated entry capacity should be `entry_count * (max/weighted)`, so the sketch is undersized by up to about 4x and never regrows, which degrades admission for weighted caches.
- **How established:** read only.

**19. Lost wakeup in async `schedule_write_op`**
- **Where:** `future/base_cache.rs:679`. The listener is created after the failed `try_send`, with no recheck.
- **What is wrong:** if the maintenance holder finishes in that window, the writer sleeps until some later operation notifies it.
- **How established:** read only. A 150 s stress of 173k rounds (`tmp/h/tests/hang.rs`) did not reproduce it, so this one is unconfirmed.

**20. Two narrow races, read only**
- `sync/base_cache.rs:1718-1737` (future from `:1877`) reads the expiration twice. If it becomes set between the reads before the wheel is enabled, `schedule` indexes empty `wheels` and panics.
- `sync/key_lock.rs` `Drop` (and `future/key_lock.rs`) can remove a lock while another thread is cloning it, so two holders of the same key would not exclude each other.

## Investigated and not reported

- **Retry exhaustion from other tasks' cancellations:** the probe `a8` produced no panic.
- **`cht` rehash and shrink:** I found no defect by reading the code.

Findings: 20. Last `./clock` reading: 20 minutes used of 45.
