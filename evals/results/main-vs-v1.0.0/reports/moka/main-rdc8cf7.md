# Audit report: moka 0.12.11 (`crate/src/`)

## Verdict

moka handles ordinary single-threaded use correctly. But two serious defects are reachable from safe code. A `Cache` with an `Expiry` policy can be driven into a use-after-free through the public API. Concurrent inserts and invalidations on the same keys permanently corrupt the cache's internal bookkeeping, until a bounded cache admits nothing at all. Defects: 2 Blocker, 4 Major, 1 Minor. Separately, there are 7 QA hygiene items.

## Blocker

### Timer wheel frees a node that the cache entry still points to
Location: `src/common/timer_wheel.rs:493-526` (`TimerEventsIter::next`).

**Problem:** this is a heap use-after-free reachable from safe code, with one thread and the public API only.

**Cause:** when a popped timer node's entry has `expiration_time() == None`, neither branch runs. The rescheduling code sits inside `if let Some(t) = expiration_time`. So the node's `Box` is dropped at the end of the loop iteration, and `unset_timer_node_in_deq_nodes()` is never called. The entry's `DeqNodes` keeps a dangling `timer_node`. The next `handle_remove` → `take_timer_node()` → `TimerWheel::deschedule` → `unlink_timer` then dereferences freed memory. The expiry can become `None` while the timer is still scheduled. That happens when `expire_after_read` returns `None` on a read whose `ReadOp` is never applied. The comment at `src/sync/base_cache.rs:333-345` names two such cases: `get_with_hash_without_recording`, and a full read channel. That comment also assumes the timer wheel will reschedule such entries, which this code does not do.

**Evidence (ran):** the reproducer is `tmp/probe/src/bin/uaf.rs`. Its `Expiry` returns 500 ms from `expire_after_create` and `None` from `expire_after_read`. The steps are:
1. `insert(1, 1)`, then `run_pending_tasks()`.
2. `get_with_if(1, || 2, replace_if)`, where `replace_if` returns true on its first call and false on its second.
3. Sleep 1.3 s, then `run_pending_tasks()`.
4. `invalidate(&1)`, then `run_pending_tasks()`.

Under `RUSTFLAGS=-Zsanitizer=address cargo +nightly run` this produced "heap-use-after-free: READ of size 1 … 0 bytes inside of 40-byte region", freed during step 3 and read during step 4. A concurrent variant using plain `get` did not reproduce in 7.3 million operations (see Unverified).

**Fix:** move the rescheduling out of the `Some` branch, so a `None` expiry reaches `schedule_existing_node`. That path already returns `Removed` and unsets the pointer:
```rust
    if let Some(t) = expiration_time {
        if t <= self.current_time { /* Expired */ }
    }
    // The entry has not expired (or has no expiry). Reschedule or remove it.
    let node_p = NonNull::new(Box::into_raw(node)).expect("Got a null ptr");
```
With exactly this change applied to a copy of the crate (`tmp/crate_patched`), the reproducer finishes cleanly under ASan. Add the reproducer as a regression test using the mock clock.

### Concurrent insert and invalidate leave "phantom" entries, and a bounded cache stops caching
Location: `src/sync/base_cache.rs:1458-1475` (`handle_upsert`) and `2011-2049` (`skip_updated_entry_ao`/`_wo`). The same code exists in `src/future/base_cache.rs`.

**Problem:** after concurrent `insert` and `invalidate`/`remove` on the same keys, `entry_count()` and `weighted_size()` grow without bound and never recover. This happens even after quiescence, `invalidate_all()` and repeated `run_pending_tasks()`. A bounded cache then believes it is full while holding nothing, and rejects every new entry. Orphaned deque nodes also leak memory.

**Cause:** write ops from different threads can reach the write-op channel in a different order from their hash-table updates. When a `Remove` is applied before the `Upsert` of the same entry, the `Remove` sees a not-yet-admitted entry and does nothing. Then `handle_upsert` admits the entry with `handle_admit` without checking that it is still in the map. The orphan node is never reclaimed: the deque scanners find its key missing and only `move_front_to_back()`. During admission, `admit` cannot find these victims in the map, so after 5 retries every candidate is `Rejected`.

**Evidence (ran):**
- `tmp/probe/src/bin/pair.rs` runs two threads on one key, unbounded, 200,000 operations each. Insert vs invalidate gave `entry_count=3695 actual=1`. Insert vs remove gave `2409 vs 1`.
- `tmp/probe/src/bin/impact.rs` uses `max_capacity(200)` with a weigher, and 4 threads mixing insert, invalidate and `entry().and_upsert_with` on 64 keys. After `invalidate_all` and 5 rounds of `run_pending_tasks`, it reported `entry_count=27 weighted_size=200 actual=0`. It then inserted 100 fresh weight-1 keys one at a time and kept 0 of 100.
- `future::Cache` showed the same failure in `tmp/probe/src/bin/fut_race.rs`: `weighted_size=201 actual=0`, and 0 of 100 fresh keys kept.

**Fix:** in `handle_upsert`, do not admit an entry that is no longer in the map. A guard that checks `cache.get(hash)` with `MiniArc::ptr_eq` on the `EntryInfo` made insert vs invalidate, insert vs remove and upsert vs invalidate exact in the patched copy. A cheaper option is to compare `gen` with the policy generation that `Remove` already records. Also make the deque scanners unlink a non-dirty node whose key is missing, instead of rotating it forever. Add the two-thread insert/invalidate test as a regression test.

## Major

### `weighted_size` drifts under concurrent updates of the same key
Location: `src/sync/base_cache.rs:524-536` (the `on_modify` closure in `do_insert_with_hash`) and `src/common/concurrent.rs:209` (`ValueEntry::policy_weight`).

**Problem:** with a weigher, concurrent `insert` calls on one key leave `weighted_size` wrong, while `entry_count` stays correct. Eviction decisions use this number, so the cache over- or under-evicts.

**Cause (traced, timing inferred):** `policy_weight` lives in the `EntryInfo` that every version of a key shares. The `on_modify` closure reads `old_weight` from it, and `new_value_entry_from` overwrites it (`set_policy_weight`, line 650) before the hash table's compare-and-swap commits. If the CAS fails, the closure retries and reads its own uncommitted weight as `old_weight`. Another thread can also read it. The delta sent to the housekeeper is then wrong.

**Evidence (ran):** `pair.rs` with two threads inserting key 0 (weights 1-5 and 1-7) gave `weighted_size=19 actual=3`, and 135 vs 5 on the patched copy. Insert vs `entry().and_upsert_with` gave 3004 vs 3. Upsert vs upsert passes, because the entry API serialises on a per-key lock. A single-threaded run of 200,000 random operations is exact.

**Fix:** keep each version's weight in its `ValueEntry`, or have the closure return the weight without mutating shared state. Update `EntryInfo::policy_weight` only when the op is applied, under the deques lock.

### `SegmentedCache` holds more entries than `max_capacity`
Location: `src/sync/segment.rs:723-724`; the docs are at `:88-89`.

**Problem:** the docs say the cache "will store up to the `max_capacity`", and `policy().max_capacity()` reports that value. The cache actually holds more.

**Cause:** the segment count is rounded up to a power of two, and each segment gets `ceil(max / segments)`.

**Evidence (ran):** `SegmentedCache::new(10, 3)` held `entry_count=12` while the policy reported `max=Some(10)`. `new(100, 64)` held 128 entries.

**Fix:** split the capacity with floor division and give the remainder to the first segments, so the per-segment capacities sum exactly to `max_capacity`. Alternatively, document the rounding and report the effective capacity.

### `unsafe` without SAFETY justification
Location: 76 `unsafe` blocks and 13 `unsafe impl`s. Examples:
- `src/common/deque.rs:56`, `128`, `139`, `169`
- `src/cht/map/bucket.rs:37`, `90`, `145`
- `src/sync/cache.rs:576`, `584`; `src/sync/segment.rs:37`, `45`; `src/future/cache.rs:641`, `649`; `src/common/iter.rs:132`, `144`

**Problem:** reviewers cannot check the invariants these blocks rely on. The timer-wheel Blocker above is exactly that kind of invariant breaking: "a timer node is freed only after its `DeqNodes` pointer is cleared".

**Assessment:** the `Send`/`Sync` impls appear sound, because every constructor requires `K, V, S: Send + Sync`. Miri passed the unit tests for deque, timer wheel, `MiniArc` and frequency sketch (16 tests).

**Fix:** add a `// SAFETY:` comment to each block stating the ownership or aliasing invariant. On each `unsafe impl Send/Sync`, state that construction requires `Send + Sync`.

### Integration tests need the network and swallow their own assertions
Location: `tests/entry_api_tokio.rs:12`, `94`, `167`, `139`, `150`, `209`, `220`.

**Problem:** the tests fetch `https://www.rust-lang.org/`, so `test_try_get_with` fails offline. Worse, the `assert!(value.is_some())` and `assert!(value.is_ok())` inside `tokio::spawn` are never checked, because `join_all(tasks)` discards the `JoinError`s.

**Evidence (ran):** offline, `test_optionally_get_with` passed even though every fetch failed. `test_try_get_with` printed 16 failures of `assertion failed: value.is_ok()` and failed only on the final call-count check.

**Fix:** replace the HTTP call with a local async function (sleep, then return a value). Unwrap every `JoinHandle` result.

## Minor

### Sync entry docs point to the async example, and the link is broken
Location: `src/sync/entry_selector.rs:201`, `743`.

**Problem:** the text reads "See [`try_append_value_async.rs`]", but the link it defines is `[try_append_value_sync.rs]`. The link therefore renders as dead text and names the wrong example. `cargo doc` with `-D warnings` fails on these two lines.

**Fix:** change the text to `try_append_value_sync.rs`.

## QA hygiene

- **Documentation:** 9 public items have no docs, including the crate root at `src/lib.rs:1` and `src/future.rs:46-58`. 21 functions lack an `# Errors` or `# Panics` section, for example `src/sync/segment.rs:279`, `288` and `src/sync/builder.rs:271`. `src/policy.rs:159`, `190` and `236` describe parameters in tables instead of prose.
- **`Debug`:** 8 public types do not implement it, including `CacheBuilder` (`src/sync/builder.rs:50`, `src/future/builder.rs:58`), the entry selectors and `Iter` (`src/common/iter.rs:22`).
- **SAFETY comments:** 89 sites. These are counted with the Major finding above, not here.
- **Panics and asserts:** 29 assertions have no message. 13 `unwrap()` calls are in production code, for example `src/common/timer_wheel.rs:328` and `src/cht/map/bucket_array_ref.rs:161`. `SegmentedCache` builders use `self.num_segments.unwrap()` (`src/sync/builder.rs:271`, `362`) where an `expect` with a stated invariant belongs.
- **Lint configuration:** `#[allow]` is used without reasons, for example `src/common/timer_wheel.rs:163` and `src/common/concurrent/arc.rs:105-106`. The 42 TODO comments have no owner, although most link an issue.
- **Private rustdoc:** `src/common/concurrent/arc.rs:83` has an unresolved link to `ptr::eq`, and `arc.rs:99` has a bare URL. Only `--document-private-items` reports these.
- **Features:** a build with no features fails by design, through a `compile_error!` that asks for `sync` or `future`. That is fine, but it means "features are additive" holds only from one enabled feature upward.

## Unverified

- **Use-after-free through plain `get`.** The Blocker should also be reachable with plain concurrent `get` calls and an `expire_after_read` that returns `None`. Either a read lands between `apply_reads` and the timer advance, or the read channel is full and the `ReadOp` is dropped. A 25-second, 4-thread run under ASan (7.3 million operations) did not trigger it.
- **Miri on the cht tests.** Miri stopped `cht::segment::tests::concurrent_growth` with a Stacked Borrows error inside `crossbeam_epoch::internal` (`&*local_ptr`), reached from `crossbeam_epoch::pin()`. That code belongs to the dependency, not moka. This report does not count it.

## Verification commands

Run from `crate/` unless noted. The probes are in `tmp/probe`; the patched copy is `tmp/crate_patched`. `crate/` itself was not modified.

- `cargo fmt --all --check`: pass.
- `cargo clippy --workspace --all-targets --features sync,future -- -D warnings`: pass.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --features sync,future`: fail. There are 4 errors: `sync/entry_selector.rs:201` and `:743`, `arc.rs:83` and `:99`.
- `cargo test --workspace --features sync,future`: fail. `tests/entry_api_tokio.rs` `test_try_get_with` needs the network; the sandbox has none. The 150 library tests and all other suites that ran passed.
- `cargo check --no-default-features`: fail by design. Every other combination passed, including `sync`, `future`, `sync,logging,quanta`, `future,unstable-debug-counters` and `--all-features`.
- `cargo +nightly miri test --features sync --lib -- common::deque common::timer_wheel common::concurrent::arc common::frequency_sketch`: pass (16 tests).
- `cargo +nightly miri test --features sync --lib -- cht::segment`: fail, inside crossbeam-epoch (see Unverified).
- In `tmp/probe`, `cargo run --release --bin pair`, `--bin race`, `--bin impact` and `--bin fut_race`: fail; these show the counter corruption.
- In `tmp/probe`, `RUSTFLAGS=-Zsanitizer=address cargo +nightly run --target x86_64-unknown-linux-gnu --bin uaf`: fail with heap-use-after-free. The same command in `tmp/probe_patched` passes.
- In `tmp/probe`, `--bin misc` shows the segmented-capacity overshoot. `main`, `stress`, `panics`, `fut_cancel` and `lfu` passed apart from the drift already reported. That covers basic operations, expiry, listeners, the entry API, predicate invalidation, panic and cancellation safety, and TinyLFU scan resistance.

## Closing summary

I covered:
- the gate and the heuristic scan;
- behaviour probes of the `sync` and `future` caches (basic operations, capacity and weigher, TTL/TTI and per-entry expiry, listeners, the entry API, predicate invalidation, the segmented cache, panic and cancellation safety, TinyLFU);
- concurrent stress tests;
- Miri on the unsafe-heavy modules;
- ASan to confirm the timer-wheel use-after-free;
- a patched copy of the crate to confirm both Blocker causes.

I did not read the lock-free hash table (`cht`) in depth beyond its main paths. I did not fuzz notification delivery modes, and I did not find a concurrent reproducer for the use-after-free. I stopped at 15 of 45 minutes. The two Blockers and the weight-drift cause were demonstrated and traced, the remaining probe areas came back clean, and further reading of `cht` was unlikely to change the verdict within the budget.

Last clock reading: 15 minutes used of 45.