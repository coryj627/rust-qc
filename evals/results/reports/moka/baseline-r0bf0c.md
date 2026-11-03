# Audit report: moka 0.12.11 (`crate/src/`)

Paths below are relative to `crate/src/`. Probes are in `<workspace>/probes/src/bin/`, where `<workspace>` is `<workspace>`.

- **Crate unchanged:** `crate/` was not modified. The in-crate probes ran on a scratch copy at `<workspace>/tmp/mk`.
- **Baseline:** `cargo test --locked --features sync,future --lib` gives 150 passed, 0 failed, 5 ignored, so the existing suite catches none of these defects.
- **Injection check:** nothing in the repository addressed me.

## High severity

### 1. Use-after-free and double free in the timer wheel: `common/timer_wheel.rs:495-526`
- **What is wrong:** `TimerEventsIter::next` has no `else` for `expiration_time() == None`. The popped node is freed without calling `unset_timer_node_in_deq_nodes()`, so `DeqNodes.timer_node` dangles. The next `deschedule` or `reschedule` touches freed memory.
- **How to reach it:** `Expiry::expire_after_read` returns `None` on a read whose `ReadOp` is not applied before the timer tick. Such reads come from `get_with_hash_without_recording`, a full read channel, or a read racing housekeeping.
- **What a caller sees:** memory corruption from safe code, or a panic at `timer_wheel.rs:283`.
- **How established (`uaf.rs`, single-threaded, using `or_insert_with_if` with a stateful `replace_if`):**
  - The debug build panicked with `internal error: entered unreachable code` at `timer_wheel.rs:283`.
  - The ASAN nightly build reported `heap-use-after-free`, with the block freed earlier on the same thread.
  - `fuzz.rs` also hit an ASAN use-after-free when `Expiry` may return `None`. Two runs (debug and ASAN, about 110 s each) were clean when it never returns `None`.
- **Limits:**
  - A probe using only `insert`, `get` and `invalidate` (`uaf2.rs`, 70 s under ASAN) did not trigger it, so the plain-`get` race is from reading only.
  - The `future` path uses the same wheel but was read, not run.
  - Miri aborted with an error inside `crossbeam_epoch::pin` on the first insert. I did not pursue that.

### 2. Ghost entries from insert and invalidate racing can make a cache useless: `sync/base_cache.rs:1458-1475`, `future/base_cache.rs:1609`
- **What is wrong:** `insert` publishes to the map and then sends its `Upsert`. If a concurrent `invalidate` sends `Remove` first, `Remove` finds the entry not admitted. The late `Upsert` then admits the already removed entry into the deques and counters.
- **Why it never clears:** eviction and TTL cannot remove a ghost, because `remove_if` finds nothing and the node just rotates.
- **What a caller sees:** `entry_count` and `weighted_size` stay high, new entries are rejected, and unbounded caches grow without limit.
- **How established (`ghost.rs`, `fut.rs`), 8 threads, 4 keys:**

| Scenario | Result |
|---|---|
| Capacity 1000, sync | map=0, `entry_count`=980, `weighted_size`=1000 |
| Same cache afterwards | retained **0 of 500** new keys (a fresh cache retained 500) |
| Unbounded | `entry_count` 13094, then 28727, then 44180, with 2-3 real entries |
| TTL 50 ms, after 200 ms idle | map=0, `entry_count`=13853 |
| `future::Cache`, capacity 1000 | map=0, `entry_count`=980 |

### 3. `future::Cache` compute operations are not atomic and lose updates: `future/value_initializer.rs:350`, `:488`
- **What is wrong:** `set_waiter_value(ReadyNone)` consumes the guard, which removes the waiter and releases the lock before `Op::Put` or `Op::Remove` is applied. The next waiter reads the old value.
- **What a caller sees:** `and_upsert_with`, `and_compute_with` and the `try` variants lose updates.
- **How established (`fcompute.rs`, 16 tasks × 2000 increments, expected 32000):**
  - `and_upsert_with` gave 31854, 31756, 31881, 31790 and 31776.
  - `and_compute_with` gave 31924.
  - The sync control (`scompute.rs`) gave 32000 every time. It holds the lock across the insert.

### 4. `invalidate_entries_if` skips entries, which become visible again: `sync/base_cache.rs:2153`, `:2170`, `future/base_cache.rs:2351`, `:2368`
- **What is wrong:** `iter.peek()` on a throwaway `Peekable<&mut Deque>` advances the deque's persistent cursor and discards that element. One entry is lost per truncated batch of 384. The head can also be skipped when a predicate outlives a scan.
- **What a caller sees:** once the predicate is removed, `get` returns entries that the predicate matched.
- **How established (`inv.rs`, `fut.rs`):** 2000 entries with `|_, _| true` left keys `[384, 769, 1154, 1539, 1924]` readable through `get`. Results were identical for sync and future.

### 5. A write over an expired entry that is not yet evicted is dead on arrival: `sync/base_cache.rs:689-705`, `future/base_cache.rs:826`
- **What is wrong:** the write is treated as an update, so `expire_after_update` runs rather than `expire_after_create`. For an expired entry `current_duration` is `None`, and the default impl returns `None`. `duration != current_duration` is false, so the past expiration time stays on the new value.
- **What a caller sees:** with an `Expiry` overriding only `expire_after_create`, `insert` followed by `get` returns `None`. `get_with` recomputes but does not retain the value, and the new value is later evicted as `Expired`. This also contradicts the `Expiry` docs, which say create is called when the key is not present.
- **How established (`exp.rs`, `fut.rs`):** re-insert then `get` gave `None`. `get_with` returned 6, then `get` gave `None` and `contains_key` gave false. Sync and future behave the same.

### 6. A key updated 65536 or more times becomes permanently dirty: `common/concurrent/entry_info.rs:97`
- **What is wrong:** in `set_policy_gen`, `current >= value` rejects wrapped values, so `policy_gen` sticks at 65535 and `is_dirty()` stays true.
- **What a caller sees:** the entry cannot be cleaned up.
  - Size eviction, TTL removal and timer expiry all skip it.
  - By reading, the `invalidate_entries_if` scan skips it too.
- **How established (`gen.rs`, `fut.rs`):**
  - After 70000 updates and TTL expiry, `entry_count` stayed 1. The control with 1000 updates reached 0.
  - Under LRU with capacity 10, the hot key survived 200 newer inserts. The control was evicted.
  - `future` gave the same TTL result.

### 7. `weighted_size` drifts under concurrent updates of a key: `sync/base_cache.rs:525`, `:650`; future `:525`, `:780`
- **What is wrong:** `on_modify` writes the new weight into the shared `EntryInfo` before the CAS. A retry or a racing thread then reads that unpublished value as `old_weight`. Saturating subtraction of out-of-order ops at `:1460` probably adds to the drift when the total is small (from reading).
- **What a caller sees:** the capacity limit is enforced against a wrong number, causing over-eviction or an unenforced limit.
- **How established (`weight.rs`, `weight2.rs`):**
  - One key of weight about 1000 reported `weighted_size` of 12173 to 792092.
  - With a 1e9-weight entry keeping the total away from zero, drift was still -663693 to +1628, so saturation is not the only cause.
  - The root cause comes from reading. The future path was read only.

## Medium severity

### 8. A panic outside `init` poisons or hangs the key in `sync::Cache`: `sync/value_initializer.rs:137`, `:153`, `:229`, `:278`
- **What is wrong:** `get()`, `insert()` and the compute read run outside `catch_unwind` and with no guard, unlike the future version. A panic in a weigher, `replace_if` or `Expiry` leaves the waiter in the map.
- **How established (`poison.rs`):**
  - Later `get_with` and `or_insert_with_if` on that key always panicked at `:127`, with `Got unexpected state Computing`.
  - After a weigher panic in `and_upsert_with`, the waiter is left as `ReadyNone`. The next upsert did not return within 5 s (the probe's timeout). From reading, it spins forever at `:220`.

### 9. A user predicate panicking during housekeeping causes lasting damage: `future/housekeeper.rs:186-196`, `sync/base_cache.rs:1293`
- **Future:** the `Shared` task stays in `current_task`, so every later maintenance run panics with `inner future panicked during poll`. The predicate had stopped panicking by then, and an ordinary `insert` and `get` also panicked.
- **Sync:** drained counters are lost when unwinding skips the `entry_count.store`. The result was `entry_count`=5 with 10 entries in the map.
- **How established:** `predpanic.rs`.

### 10. `try_get_with::<_, ()>` makes a concurrent `get_with` panic: `sync/value_initializer.rs:348`, `sync/cache.rs:1053`, `future/cache.rs:1597`
- **What is wrong:** both calls use `TypeId::of::<()>()` as the waiter key. `get_with` reads the other call's `Err(())` and hits `InitResult::InitErr(_) => unreachable!()`.
- **How established (`unit_err.rs`, sync):** it panicked at `cache.rs:1053`. The future path has the same code but was read only.

### 11. `entry_by_ref(..).or_try_insert_with(..)` loses the key on a cache hit: `sync/cache.rs:1422`
- **What is wrong:** `need_key` is ignored and `false` is passed, so a hit returns an `Entry` with no key.
- **What a caller sees:** `.key()` panics with `Bug: Key is None`, and `Debug` on the entry calls `.key()` too.
- **How established:** `byref.rs`. The future counterpart at `future/cache.rs:1763` passes `need_key` correctly (read).

### 12. An untracked entry can stay in the map: `sync/base_cache.rs:539-546`, `future/base_cache.rs:541-546`
- **What is wrong:** the sequence is insert CAS fails, modify CAS fails, then a tombstone reuses the earlier insert value without calling `on_insert`. `cnt2 > cnt1` then selects the update op, so the entry actually in the map never gets a `WriteOp`.
- **What a caller sees:** an entry that is never admitted or counted, and with no TTL, size or timer cleanup.
- **How established:** an in-crate stress test on the scratch copy found 1 of 64 live keys not admitted after all ops were applied. That this interleaving is the cause comes from reading.

### 13. `remove` returns expired or invalidated values: `sync/cache.rs:1601`, `future/cache.rs:1959`
- **What is wrong:** `remove_entry` has no validity check, so `get` and `remove` disagree.
- **How established (`misc.rs`, sync):** `remove` gave `Some(1)` after TTL expiry while `get` gave `None`. After `invalidate_all`, `get` gave `None` and `remove` gave `Some(1)`. The future path has the same call but was read only.

## Low severity

### 14. Wrong `RemovalCause` for per-entry expiry: `sync/base_cache.rs:2306-2355`, `future/base_cache.rs:2513`, `:2554`
`notify_upsert` and `notify_invalidate` ignore `expiration_time`. In `misc.rs`, per-entry expired entries reported `Replaced` and `Explicit`, while the TTL control reported `Expired` for both.

### 15. `is_old_value_replaced()` is wrong for `or_insert_with_if`: `sync/cache.rs:1050`, `future/cache.rs:1594`
When `or_insert_with_if` replaces a value, the flag is `false`, contrary to its docs. `replaced.rs` showed value=2, fresh=true, old_replaced=false.

### 16. `invalidate_all` disagrees with its documentation: `sync/base_cache.rs:2463`, `future/base_cache.rs:2694`
The docs at `sync/cache.rs:1647-1648` promise entries inserted "before or at" the invalidation time are not returned, but the code uses `entry_ts < va`. A mock-clock test on the scratch copy got `Some(1)`. This needs equal timestamps, so a real ns clock will rarely show it.

### 17. Already-expired timers go into a past bucket: `common/timer_wheel.rs:335-346`
The bucket index uses `time` even when it is earlier than `self.current`, so the timer waits for a wheel lap. Eviction, the listener call and the `entry_count` update are delayed by up to about 69 s, while `get` already hides the entry. An in-crate test fired after 58 s for a time 10 s in the past. That a delayed write op reaches this path through the public API comes from reading.

### 18. Sketch sizing is inverted with a weigher: `sync/base_cache.rs:1348`, `future/base_cache.rs:1488`
The code uses `entry_count * (weighted/max)` where capacity estimation needs `entry_count * (max/weighted)`, so the sketch is undersized by up to about 4x and admission quality drops. Read only.

### 19. Predicate registration race on `is_empty`: `sync/invalidator.rs:101-102`, `:264-265` against `:130`
A housekeeper's `store(true)` can overwrite a concurrent registration's `store(false)`. That predicate is then ignored by reads and scans until another one is registered. Read only.

### 20. `KeyLock` removal race: `sync/key_lock.rs:30-38`, same in `future/key_lock.rs`
Another thread can clone the lock between the `count <= 2` check and the CAS. Later callers then get a different mutex, so per-key notification exclusion can fail. Read only.

## Checked and not reported
I suspected a lost wakeup in `future/base_cache.rs` `schedule_write_op`, because `listen()` follows `try_send` with no recheck. `wake.rs` ran up to 150 rounds with no stall, so it is unproven.

Findings: 20. Last `./clock` reading: 26 minutes used of 45.
