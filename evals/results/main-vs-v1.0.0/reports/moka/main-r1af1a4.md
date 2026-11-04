# moka 0.12.11 audit report

## Verdict

The basic features behave correctly on single-threaded inputs. Probes of eviction, TTL, TTI, per-entry expiry, invalidation, listeners, the entry API and a 20-seed random model test all match a reference. The real problems show up under concurrency and at a few API edges. The size counters drift permanently under ordinary multithreaded use, and a legitimate mix of `get_with` and `try_get_with` panics. Defects: 0 Blocker, 6 Major, 3 Minor, 1 Note. QA hygiene items: 6.

## Major

### Size counters drift permanently under concurrent writes, shrinking bounded caches
Location: `src/sync/base_cache.rs:1458-1475` (`handle_upsert`) and `:1779-1787` (`handle_remove_without_timer_wheel`). The same code is in `src/future/base_cache.rs:1609-1626`.

Problem: `entry_count()` and `weighted_size()` end up above the real contents. Nothing ever corrects them, even after `run_pending_tasks`. The excess grows with uptime. In a bounded cache the phantom weight takes up real capacity, so the cache holds fewer entries than `max_capacity`.

Cause: each thread puts its write ops on a shared channel, and they can be applied in a different order from their hash-table effects. I confirmed both of the following races in isolation.
- An entry's Remove is applied before its Upsert. The Remove sees `is_admitted() == false` and only clears the queue nodes. The late Upsert then admits an entry that is no longer in the map, which adds 1 to the count for good.
- An update's Upsert is applied before the insert's Upsert for the same `EntryInfo`. The update admits the entry. The insert op then sees it already admitted and adds its weight again.

Evidence (ran):
- Unbounded `sync::Cache`, 4 threads, 75% insert and 25% invalidate over 2,000 keys. The phantom count grew with run length:

  | Ops per thread | Phantom entries | Phantom weight |
  |---|---|---|
  | 100,000 | 60 | 70 |
  | 400,000 | 252 | 280 |
  | 1,600,000 | 1000 | 1135 |

- After invalidating every real entry: `iter` 0, `entry_count` 1000, `weighted_size` 1135.
- One inserter and one invalidator, each key inserted once: real 194054, `entry_count` 194055.
- Two inserters, nothing removed: real 400000, `weighted_size` 400001.
- Bounded at 500 with a weigher: `weighted_size` 500, real weight 370.
- `future::Cache` with the same workload: real 1517, `entry_count` 1692.
- With a single thread there is no drift.

Fix: drop stale ops instead of applying them.
- Before admitting in `handle_upsert`, check that the map still holds this `EntryInfo`, the same `ptr_eq` and `entry_gen` test the reject path already uses.
- Have a not-admitted Remove mark the `EntryInfo` as removed, so a later Upsert is discarded.
- Skip any Upsert whose `gen` is older than the `policy_gen` already applied.

### `get_with` panics when it shares a key with a failing `try_get_with` whose error type is `()`
Location: `src/sync/value_initializer.rs:345-349` (`type_id_for_get_with`), panic at `src/sync/cache.rs:1053`. Async: `src/future/value_initializer.rs:589-593`, panic at `src/future/cache.rs:1597`.

Problem: if `get_with` (or `entry().or_insert_with`) runs on the same key as an in-flight `try_get_with::<_, ()>` that fails, it panics with "entered unreachable code".

Cause: waiters are keyed by `(key, TypeId)`. `get_with` uses `TypeId::of::<()>()`, which is the same id as `try_get_with` with `E = ()`. So `get_with` joins that waiter, reads `Ready(Err(..))`, and the downcast to `()` succeeds. It then hits `InitResult::InitErr(_) => unreachable!()`. The docs say calls are only coalesced when they share an error type, so these two should never meet.

Evidence (ran): thread A runs `try_get_with(1, || { sleep(300ms); Err(()) })`, and thread B runs `get_with(1, || 42)` 50 ms later. B panics at `sync/cache.rs:1053:39`. The tokio version panics at `future/cache.rs:1597:39`.

Fix: give `get_with` a private marker type, for example `struct GetWith; TypeId::of::<GetWith>()`, the way `OptionallyNone` already works.

### `remove` returns values that are expired or invalidated
Location: `src/sync/cache.rs:1601-1623` and `src/future/cache.rs:1959-1968` (`invalidate_with_hash`).

Problem: `get` and `contains_key` treat an entry as gone once it has expired or been hit by `invalidate_all`. `remove` still hands its value back, so code that uses `remove` as "take" receives stale data.

Cause: `remove_entry` removes whatever is in the map, and `Some(kv.entry.value.clone())` is returned with no expiry or `valid_after` check.

Evidence (ran): with TTL 50 ms, after 100 ms `get` returns None and `contains_key` returns false, but `remove` returns `Some(1)`. The same happens after `invalidate_all`, and in `future::Cache`.

Fix: apply the same checks `do_get_with_hash` uses before returning the value. Still remove the entry and notify as `Expired` or `Explicit`.

### `SegmentedCache` holds more entries than `max_capacity`
Location: `src/sync/segment.rs:721-724`.

Problem: the docs say the cache "will store up to the `max_capacity`". It actually holds up to 60% more.

Cause: the segment count is rounded up to a power of two, and each segment gets `ceil(max / segments)`. The total is therefore `segments * ceil(max / segments)`.

Evidence (ran): after 10,000 inserts, `run_pending_tasks` and `policy().max_capacity()`:

| `max_capacity` | Segments | `entry_count` |
|---|---|---|
| 10 | 8 | 16 |
| 100 | 64 | 128 |
| 5 | 3 | 8 |

Fix: give each segment `max / segments` and spread the remainder across the first segments. Alternatively, document the rounding and the effective bound.

### `or_insert_with_if` reports `is_old_value_replaced() == false` after replacing a value
Location: `src/sync/cache.rs:1050` and `src/future/cache.rs:1594`.

Problem: `Entry::is_old_value_replaced` is documented as true "if an old value existed in the cache and was replaced". When `replace_if` fires, the old value is replaced but the flag is false.

Cause: `insert_with_hash_and_fun` always builds `Entry::new(k, v, true, false)`.

Evidence (ran): `insert(1, 1)`, then `entry(1).or_insert_with_if(|| 5, |v| *v == 1)` gives value 5, fresh true, replaced false, and `get` returns 5. `future` behaves the same.

Fix: pass through whether `replace_if` matched an existing value, and set the flag from that.

### Undocumented unsafe code: 76 blocks and 13 `unsafe impl`s
Location: across the crate. Key ones:
- `src/common/deque.rs` (all raw-node operations)
- `src/cht/map/bucket.rs:37-40` (zeroed `Atomic` buckets)
- `src/common/timer_wheel.rs:167` (`unsafe impl Send`)
- `src/sync/cache.rs:576-590`
- `src/future/cache.rs:641-655`
- `src/sync/segment.rs:37-49`

Problem: most `unsafe` has no SAFETY reasoning.
- Several `unsafe fn`s (`Deque::move_to_back`, `unlink`, `unlink_and_drop`) have no `# Safety` section.
- The `unlink_and_drop` doc has an empty "Panics:" line.
- The public `Send` impls require only `S: Send` where sharing a cache needs `&S` across threads. I found no safe exploit, because every method that touches `S` requires `S: Sync`. But the reasoning is not written down.

Evidence (ran): the clippy advisory pass reported 76 unsafe blocks and 13 unsafe impls missing safety comments. Miri passed the deque, `MiniArc`, timer wheel and frequency sketch tests.

Fix: add SAFETY comments that state the ownership and aliasing invariants, plus `# Safety` sections. Consider requiring `S: Send + Sync` on the public `Send` impls.

## Minor

### A huge `initial_capacity` panics with arithmetic overflow
Location: `src/cht/segment.rs:178`.

Problem: `capacity * 2 / actual_num_segments` overflows. The builder does not document this panic.

Evidence (ran): `Cache::builder().initial_capacity(usize::MAX / 2 + 1).build()` panics with "attempt to multiply with overflow" in a debug build.

Fix: use `saturating_mul`, cap the per-segment length, and document the limit.

### The sync `and_try_compute_with` docs link to the async example, and the link is broken
Location: `src/sync/entry_selector.rs:201` and `:743`.

Problem: the text says ``[`try_append_value_async.rs`]``, but the link definition below it is for `try_append_value_sync.rs`. Rustdoc reports an unresolved link and renders broken text.

Fix: change the text to `try_append_value_sync.rs`.

### Several public methods lack docs
Location: `src/sync/segment.rs:279` and `:288` (`SegmentedCache::entry` and `entry_by_ref`), `src/notification.rs:44`, `src/future/entry_selector.rs:242` and `:897`, the `policy` module (`lib.rs:265`), and `future::FutureExt` (`future.rs:46`).

Fix: document them in the same way as their `Cache` counterparts.

## Note

### `PredicateError` is an exhaustive public enum
Location: `src/common/error.rs:8`.

Adding a variant later would be a breaking change. Consider `#[non_exhaustive]` at the next breaking release.

## QA hygiene

- **Rustdoc gate fails (4 errors).** The gate runs with `-D warnings --document-private-items`. Two errors are the broken link above. The others are a `ptr::eq` intra-doc link at `common/concurrent/arc.rs:83` and a bare URL at `arc.rs:99`.
- **Documentation sections.** 20 fallible public functions lack an `# Errors` section, and `Entry::key` (`common/entry.rs:58`) lacks a `# Panics` section.
- **Missing `Debug` (8 public types).** These are both `CacheBuilder`s, the entry selectors (`sync/entry_selector.rs:22` and `:560`, `future/entry_selector.rs:23` and `:674`), `Iter` (`common/iter.rs:22`) and `future.rs:58`.
- **`unwrap()` outside tests (13).** Most guard real invariants, such as `housekeeper.rs:102` `run_after.instant().unwrap()`, `timer_wheel.rs:328`, and the waiter downcasts in the `value_initializer.rs` files. They should be `expect` calls with a reason.
- **Lint configuration.** There are 59 `#[allow]`s without a reason and 29 assertions without messages. The CHANGELOG says `non_send_fields_in_send_ty` was removed, but it is still allowed at `timer_wheel.rs:163` and `deques.rs:19`.
- **Heuristic scan.** It found 3 parameter tables in the `Expiry` docs (`policy.rs:159, 190, 236`) and about 40 TODOs without issue references. Some of those TODOs flag known races, such as `sync/cache.rs:1587`.

## Unverified

- **`Send`/`Sync` bounds** on `Cache` and `SegmentedCache` (`S: Send` for `Send`): I found no safe-code path to a data race. See the unsafe finding.
- **Miri, Stacked Borrows:** it reports undefined behaviour in `cht::segment::tests::drop_value`, but the violation is inside crossbeam-epoch 0.9.21 (`internal.rs:567`, its intrusive `Local` list during `pin()`), not in moka code.
- **Miri, Tree Borrows:** the moka `cht` unsafe code is still unverified under Miri.
  - The first run of the same tests finished. But I had cut its output short, and the saved part shows only crossbeam-epoch integer-to-pointer warnings, with no error and no result line.
  - The rerun hit my 800-second timeout before printing a result.

## Verification commands

All commands run from `crate/` unless stated otherwise.

- `cargo fmt --all --check`: pass.
- `cargo clippy --workspace --all-targets --features sync,future -- -D warnings`: pass.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --features sync,future`: fail, 4 errors (listed above).
- `cargo test --workspace --features sync,future`: pass. Lib tests 150 passed with 5 ignored; integration tests and doctests passed, including 67 passed with 2 ignored.
- `cargo +nightly-2026-10-01 miri test --features sync --lib -- common::deque common::concurrent::arc common::timer_wheel common::frequency_sketch`: pass, 16 tests.
- `cargo +nightly-2026-10-01 miri test --features sync --lib -- cht::segment::tests::drop_value`: fail, Stacked Borrows violation inside crossbeam-epoch.
- `MIRIFLAGS=-Zmiri-tree-borrows cargo +nightly-2026-10-01 miri test --features sync --lib -- cht::segment::tests::{single_segment,insertion,removal,insert_with_or_modify,drop_value} cht::map::bucket::tests`: inconclusive (truncated output, then a timeout).
- Probes: a throwaway crate in `tmp/probe` with a path dependency on `crate`, built with `cargo run --offline --bin p2` through `p14`. The concurrency probes use `--release`. The failing ones are `p3` (panic), `p5` (`remove` and `or_insert_with_if`), `p7` (segment capacity), and `p9` through `p13` (counter drift).

## Closing summary

I covered the gate, the heuristic scan, and probes of eviction (TinyLFU and LRU, weigher), TTL, TTI and `Expiry`, invalidation, listeners, the entry and compute APIs, `SegmentedCache`, the async cache, a seeded model test and multithreaded stress. I also read the timer wheel, deque, frequency sketch, `MiniArc`, value initializer, key lock, invalidator and the core of `cht`.

I left some things unchecked:
- the `cht` rehash and concurrency paths under Miri: the Tree Borrows rerun timed out, and another one would not fit in the remaining time;
- the `quanta` and `logging` features;
- builds at the MSRV of 1.70;
- line-by-line reading of the 5,000-line `cache.rs` files.

I also could not tell which defects predate this release and which are local changes, since there is no git history to compare against. Nothing in the repository or its output tried to give me instructions. I stopped at 33 minutes to leave time for the report.

Final clock: 33 minutes used of 45.