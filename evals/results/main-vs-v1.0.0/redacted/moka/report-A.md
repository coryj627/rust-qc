# Audit report: moka 0.12.11 (`crate/src/`)

## Summary

moka 0.12.11 builds, passes clippy with `-D warnings`, and passes its whole test suite with `sync,future`. The core admission, eviction, expiry and timer-wheel logic behaved correctly in every probe. The real defects are in two areas. First, `sync::Cache` handles panicking user callbacks badly: one panic can poison a key for good or stop maintenance for good. Second, some API behavior disagrees with what `get` and the docs promise. 

## Medium

**A panicking weigher poisons a key in sync get_with and makes and_compute_with spin forever**
`src/sync/value_initializer.rs:148-173` (`try_init_or_read`) and `:241-300` (`try_compute`).
`catch_unwind` only wraps `init` or `f`. If the later `insert` panics (for example, a weigher that panics on some value), the waiter is never removed from the map:
- In `get_with` the waiter stays `Computing`. Every later `get_with` or `entry(k).or_insert_with` on that key panics with "Got unexpected state `Computing` ... This might be a bug in Moka".
- In `and_compute_with` the waiter stays `ReadyNone`. The next compute on that key loops forever on `continue` (line 220) and burns a CPU.

Reproduced with a weigher that panics on the value 13. `future::Cache` recovers correctly, because it uses a `WaiterGuard` drop guard.

**Recursive get_with or and_compute_with on the same key deadlocks silently**
`src/sync/value_initializer.rs:110` and `:211`.
if an `init` closure calls `cache.get_with(same_key, ..)`, the inner call takes a read lock on the waiter whose write lock the same thread already holds. `c.get_with(1, || c2.get_with(1, || 2))` hangs forever, and so does the matching `and_compute_with` case (both checked with a 3 s timeout). Nothing in the docs warns about this. Caffeine fails fast in the same situation ("Recursive update").

**A panicking invalidation predicate stops all maintenance for good**
`src/sync/invalidator.rs:186` (`scan_and_invalidate`) and `:143` (`apply_predicates`).
after `invalidate_entries_if` registers a predicate that panics on one value, the panic escapes every time:
- `get` panics on that entry.
- Every `run_pending_tasks` panics, including runs triggered from inside `insert`.

Because maintenance never completes, the predicate is never removed, eviction and expiry stop, and the counters are never stored. In the probe, `entry_count()` stayed at 0 with 11 live entries. The capacity bound is no longer enforced, so memory can grow without limit.

**remove() returns values that get() treats as expired or invalidated, and reports them as Explicit**
`src/sync/cache.rs:1601-1623` (`invalidate_with_hash`), `src/sync/base_cache.rs:2336` (`notify_invalidate`), and `src/future/base_cache.rs:2554` (same logic in the future cache).
`invalidate_with_hash` never checks whether the entry is expired or invalidated. Observed:
- With a TTL, `get(&2)` returns `None` after expiry, but `remove(&2)` then returns `Some(2)`.
- After `invalidate_all()`, `remove(&1)` returns `Some(1)`.

`notify_invalidate` checks only the cache-wide TTI and TTL. So an entry expired by the per-entry `Expiry`, or hidden by `invalidate_all`, reaches the eviction listener as `RemovalCause::Explicit` instead of `Expired`.

**SegmentedCache with a weigher silently rejects entries that fit within max_capacity**
`src/sync/segment.rs:723-724`.
`max_capacity` is split evenly across segments, and each segment rejects anything heavier than its share. In `SegmentedCache::builder(4).max_capacity(100).weigher(len)`, a 30-byte value is never stored (`contains_key` is false, `weighted_size` is 0), even though it is well under 100. Nothing in the docs mentions the split.

**Unsafe code lacks SAFETY justification**
76 `unsafe` blocks and 13 `unsafe impl`s across the crate. The most important are `src/common/deque.rs:56`, `:128`, `:139` and `:169`; `src/common/concurrent/deques.rs:23` and `:79`; `src/common/concurrent.rs:156`; `src/common/timer_wheel.rs:167`; and the `Send`/`Sync` impls in `src/sync/cache.rs:576-590`.
the code I read (Deque, MiniArc, timer-wheel rescheduling) looks sound, but most blocks have no `// SAFETY:` comment. The `Send` impls for `Deques`, `DeqNodes` and `TimerWheel` rely on the cache's locking discipline, and nothing written down states that.

## Low

**SegmentedCache can hold more entries than max_capacity**
`src/sync/segment.rs:723`.
each segment's share is rounded up with `ceil`. `SegmentedCache::new(10, 16)` held 16 live entries, while `policy().max_capacity()` reported `Some(10)` and the docs say it "will store up to the `max_capacity`".

**Overflow panics on extreme builder values**
`src/sync/base_cache.rs:1009` (`cap + WRITE_LOG_CH_SIZE`) and `src/sync/segment.rs:721` (`next_power_of_two`).
in debug builds, `initial_capacity(usize::MAX)` and `SegmentedCache::new(10, usize::MAX)` panic with "attempt to add with overflow". In release builds the arithmetic wraps silently. Neither case is documented.

**Rustdoc fails with -D warnings: wrong example link in public docs**
`src/sync/entry_selector.rs:201` and `:743`, plus `src/common/concurrent/arc.rs:83` and `:99`.
the sync `and_try_compute_with` docs say "See [`try_append_value_async.rs`]", but the link that follows defines `try_append_value_sync.rs`. Readers are sent to the wrong example and the link is broken. The two `arc.rs` errors are private-item doc nits.

**invalidate_all docs say "before or at" but the code uses a strict comparison**
`src/sync/base_cache.rs:2463` (`entry_ts < *va`); the docs are at `src/sync/cache.rs:1646`.
an entry stamped with the same nanosecond as the `invalidate_all` call survives, which contradicts "inserted before or at the invalidation time".

**Lint and documentation hygiene**
across the crate.
the advisory lint pass reported:
- 29 asserts without a message
- 20 functions returning `Result` without an `# Errors` section
- 13 `unwrap` calls outside tests, for example `src/sync/builder.rs:271` and `:362` (`num_segments.unwrap()`); these are invariant-backed, so `expect` would be better
- 8 types without `Debug`
- 9 missing-docs warnings
- 3 parameter tables in `src/policy.rs:159`, `:190` and `:236`
- a `dbg!` in the test-only mock clock at `src/common/time/clock.rs`
- `#[allow]` attributes without reasons

## Low

- Miri: under Stacked Borrows, `cht::segment::tests::concurrent_growth` reports a violation inside crossbeam-epoch 0.9.21's own list code (`internal.rs:567`), not in moka's code. A Tree Borrows rerun of the `common::` and `cht::` lib tests had produced no results when I stopped.
- The timer wheel works at about 1.07 s granularity. Expired entries stay counted in `entry_count()` until the next tick, even after `run_pending_tasks` (I saw this with 200 ms per-entry expiries). `get` hides them correctly. Worth a sentence in the `Expiry` and `entry_count` docs.
- About 40 TODO comments have no owner, although most link to a GitHub issue. Nothing in the repository text or the probe output tried to address me as an instruction.
