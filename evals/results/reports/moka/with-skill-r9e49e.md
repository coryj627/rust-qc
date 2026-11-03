# Audit: moka 0.12.11

Verdict: the crate builds and its own test suite passes, but it has real defects in the paths those tests skip. Safe code can reach a use-after-free through the timer wheel. Ordinary concurrent insert and invalidate calls can permanently corrupt a cache's accounting, until it retains nothing. The async compute guarantee does not hold, and `invalidate_entries_if` leaves entries behind. Findings: 2 Blocker, 10 Major, 12 Minor.

Unless a finding says "read only", I reproduced it through the public API from a throwaway crate in `probes/p1/src/bin/`, run with `./run`. `sync` and `future` share most of this logic, so I give both locations where they apply. `crate/` was not changed. Internal probes used a copy at `probes/copy`.

## Blocker

### Use-after-free: timer node freed while its pointer stays in the entry
Location: `src/common/timer_wheel.rs:493-526`. The stale pointer is dereferenced at `timer_wheel.rs:276-279`, reached through `sync/base_cache.rs:1753-1755` and `1767-1768` (`future/base_cache.rs` has the same paths).

Problem: `TimerEventsIter::next` pops a node and only handles `if let Some(t) = expiration_time`. If an `Expiry` has cleared the expiration by then, the box is dropped without `unset_timer_node_in_deq_nodes`, so `DeqNodes.timer_node` dangles. The next reschedule or removal then reads and writes freed memory. Clearing can happen when `expire_after_read` returns `None` during maintenance, when the `ReadOp` is discarded because the channel is full, or when a read is not recorded. The comment at `sync/base_cache.rs:342-345` says these cases are okay; they are not.

What a user sees: memory corruption or a crash from safe code.

Established by:
- `uaf_st.rs` is single-threaded: `expire_after_read` returns `None`, `get_with_if` makes a read that is not recorded, then sleep, `run_pending_tasks` and `invalidate`.
  - AddressSanitizer reports heap-use-after-free.
  - The release build panics with `index out of bounds: the len is 5 but the index is 175` at `timer_wheel.rs:279`, which is garbage read from the freed node.
- `timer_uaf.rs` does it with two threads and `get`, holding maintenance open through a blocking eviction listener. AddressSanitizer reports heap-use-after-free and the release build dies with `Bus error`.

Fix: add an `else` branch that calls `unset_timer_node_in_deq_nodes` and returns `TimerEvent::Descheduled`.

### Removed entries get admitted as permanent phantoms, and the cache stops retaining anything
Location: `src/sync/base_cache.rs:1458-1475` and `1785-1787`, `src/future/base_cache.rs:1609` onward, with op sending at `sync/cache.rs:1487-1489` and `1601-1637`.

Problem: `insert` changes the map and sends its `WriteOp` as two separate steps, and so does `invalidate`, so a `Remove` can be applied before the matching `Upsert`. `handle_upsert` then sees "not admitted" and admits an entry that is no longer in the map. The phantom is counted and stays in the deques. Eviction can never remove it, because `remove_if` returns `None` and `skip_updated_entry_*` only rotates it to the back.

What a user sees, with 4 threads doing `insert`/`invalidate` on 16 keys (`phantom.rs`, `phantom2.rs`, `phantom_fut.rs`):
- A capacity-1000 cache ends with `entry_count=970 weighted=1000 actual=0`.
- Inserting 1000 fresh keys afterwards retains 0. The control without churn retains 1000.
- An unbounded cache grows without limit: `entry_count` went 14343, 29260, 45682, 62094 over 4 rounds, with 0 real entries.
- A 100 ms TTL does not clear the phantoms.
- `future::Cache` behaves the same: `entry_count=954`, 0 of 1000 retained.

Fix: mark removal on `EntryInfo` when an entry leaves the map, and have `handle_upsert` skip entries so marked. Alternatively, check that the map still holds this `EntryInfo` before admitting.

## Major

### Async compute loses updates despite the documented serialization
Location: `src/future/value_initializer.rs:350` and `488` (documented at `future/entry_selector.rs:163`).

Problem: `set_waiter_value(ReadyNone)` consumes the guard, which releases the lock and removes the waiter before the returned `Op` is applied. A second compute can therefore read the old value.

What a user sees (`compute_atomic.rs`, 8 tasks, 800000 increments):
- `and_compute_with` gave 799916.
- `and_upsert_with` gave 799881.
- `sync` gave exactly 800000.

Fix: hold the guard until the `Op` is applied, as the sync version does.

### `invalidate_entries_if` skips the entry at every batch boundary
Location: `src/sync/base_cache.rs:2153` and `2170`, `src/future/base_cache.rs:2351` and `2368`.

Problem: `iter.peek()` on a `Peekable<&mut Deque>` advances the deque's own cursor, and the peeked element is discarded. Once the predicate finishes it is removed, so skipped entries become readable again.

What a user sees (`inv.rs`, `fut_misc.rs`): after a match-everything predicate on 2000 entries, survivors are `[384, 769, 1154, 1539, 1924]`, and `contains_key` returns true for them. Sync and future agree.

Fix: check for more entries without consuming one, for example with a method on `Deque`.

### A second predicate starts from a stale scan cursor
Location: `src/sync/base_cache.rs:2170` and `2184-2186`.

Problem: the cursor is reset only when no predicates remain. If a new predicate is waiting when the current one finishes, its scan continues from the old position. The trailing `peek()` after exhaustion has also already restarted the iterator and consumed the head.

What a user sees (`inv.rs`): with a first predicate in progress and a second match-everything predicate, survivors are `[0, 385, 770, 1155]` of 1200, and key 0 is the head skipped here. A predicate that finishes part way through the deque would also skip everything before the cursor; that wider case is from reading only, because my probe for it (`inv2.rs`) did not reach it.

Fix: reset the cursor whenever the scan context is reloaded with new predicates.

### After about 65536 updates of one key, the entry stays dirty almost permanently
Location: `src/common/concurrent/entry_info.rs:97`.

Problem: `current >= value` rejects the wrapped generation. With `policy_gen` at 65535 and `entry_gen` wrapped to 1, the update never lands, so `is_dirty()` stays true. Dirty entries are skipped by predicate scans, TTL eviction and size eviction.

What a user sees (`gen.rs`, 65540 updates):
- A match-all predicate leaves `key1=Some(65539)` readable.
- After TTL expiry `entry_count` stays 1; the 60000-update control reaches 0.
- In an LRU cache of capacity 1, the hot key stays present while 190 newer keys are evicted.

Fix: compare with wrapping arithmetic only: `value.wrapping_sub(current)` in `1..=u16::MAX/2` means advance.

### Updating an entry whose per-entry expiry has passed leaves the new value expired
Location: `src/sync/base_cache.rs:689-706`, `src/future/base_cache.rs:826`.

Problem: `checked_duration_since` gives `None` for a past expiration, so `duration != current_duration` is false when the `Expiry` returns `None`. That is what the default `expire_after_update` returns in this case, and it is also how a user asks for no expiration. The old past timestamp is kept.

What a user sees (`expiry.rs`): after the old expiry passes, `insert(1, 2)` followed by `get(&1)` returns `None`, both with the default update method and with one that explicitly returns `None`.

Fix: treat an already-expired entry as a create, or always write the result when the stored expiration is in the past.

### `weighted_size` drifts under concurrent updates with changing weights
Location: `src/sync/base_cache.rs:525` and `650`, `src/future/base_cache.rs:525` and `780`.

Problem: the retryable `on_modify` closure reads `old_weight` from the shared `EntryInfo`, then overwrites it. Failed or racing attempts mutate that value, so the deltas no longer add up.

What a user sees (`weight.rs`): two keys finishing at weight 5 each report `weighted_size` of 87427, 75939 and 3176 where 10 is expected. A bounded cache would then evict against a wrong size.

Fix: keep the weight on the immutable `ValueEntry`, or commit it to `EntryInfo` only after the compare-exchange succeeds.

### `get_with` panics when it waits on `try_get_with` with error type `()`
Location: `src/sync/value_initializer.rs:348` with `sync/cache.rs:1053`, `src/future/value_initializer.rs:592` with `future/cache.rs:1597`.

Problem: `get_with` uses `TypeId::of::<()>()` as its waiter key, the same key `try_get_with::<_, ()>` uses. The error reaches `InitErr(_) => unreachable!()`.

What a user sees (`waiter.rs`, `fut_misc.rs`): `internal error: entered unreachable code` in both caches, on a caller that did nothing wrong.

Fix: use a private marker type for the `get_with` key.

### Panic outside the guarded closure poisons a key in `sync::Cache`
Location: `src/sync/value_initializer.rs:137`, `153` and `278`.

Problem: only `init` and `f` are wrapped in `catch_unwind`. A panic in `insert` (a weigher, for example) or in `get` leaves the waiter in the map. The async version has a drop guard for this.

What a user sees (`waiter.rs`, `misc.rs`):
- After one panicking weigher call, every later `get_with` on that key panics with `Got unexpected state Computing`, even after `invalidate`.
- After a panic in `and_upsert_with`, the next `and_upsert_with` on that key had still not returned after 3 s. From the code it spins in the retry loop.

Fix: add a drop guard like `WaiterGuard` that removes the waiter on unwind.

### `remove` returns values that were already invalidated or expired
Location: `src/sync/cache.rs:1601-1623`, `src/future/cache.rs:1959`.

Problem: no validity check is made. The `invalidate_all` docs (`sync/cache.rs:1646-1648`) promise retrieval methods will not return such entries. From the code, `and_compute_with` returning `Op::Remove` goes through the same path and can report `Removed` with a stale value; my compute probe got `StillNone` because maintenance ran first.

What a user sees (`misc.rs`, `weight.rs`):
- After `invalidate_all`: `get=None`, `remove=Some(1)`.
- With a match-all predicate registered: `get=None`, `remove=Some(1)`.
- After TTL expiry: `remove=Some(1)`.

Fix: apply the same expiry and invalidation checks as `get`, and return `None` when they fail.

### Removal listener reports the wrong cause for per-entry expiry
Location: `src/sync/base_cache.rs:2306-2333` and `2336-2355`, `src/future/base_cache.rs:2513` and `2554`.

Problem: only TTL and TTI are checked. `OldEntryInfo` does not record the expiration time.

What a user sees (`expiry.rs`): an entry past its `Expiry` time is reported as `Replaced` on overwrite and `Explicit` on invalidate. `RemovalCause::Expired` is documented for exactly this case.

Fix: capture the expiration before the update and check `is_expired_by_per_entry_ttl`.

## Minor

### Expiry set by an unrecorded read never gets a timer
Location: `src/sync/base_cache.rs:342-345`.

Problem: when the `ReadOp` is lost and the entry has no timer yet, nothing ever evicts it.

What a user sees (`notimer.rs`): `get=None`, `entry_count=1`, indefinitely.

Fix: have the scheduling step pick up the missing timer, or retry the lost op on the next write or read of that entry.

### `or_insert_with_if` reports no replacement when it replaced a value
Location: `src/sync/cache.rs:1050`, `src/future/cache.rs:1594`.

What a user sees (`misc.rs`): `value=2 fresh=true old_replaced=false`, which contradicts `Entry::is_old_value_replaced`. Established by running the sync path; the future location is from reading.

Fix: pass whether the `replace_if` path found an existing value.

### `invalidate_all` misses entries from the same clock tick
Location: `src/sync/base_cache.rs:2463`, `src/future/base_cache.rs:2694`.

Problem: `entry_ts < va` is strict, but the docs promise "before or at".

What a user sees (mock clock test in `probes/copy`): `get` returns `Some(1)`.

Fix: use `<=` and handle inserts in the same tick, or correct the docs.

### One `run_pending_tasks` processes only one predicate batch
Location: `src/sync/base_cache.rs:1272-1274` and `2181-2189`.

Problem: `invalidate_entries` never increments `eviction_count`, so the loop stops after one batch.

What a user sees (`misc.rs`): `entry_count=1616` of 2000 after one call, while the same call after `invalidate_all` gives 0.

Fix: count these removals as evictions.

### Frequency sketch size formula looks inverted for weighted caches
Location: `src/sync/base_cache.rs:1348`, and the same formula at `src/future/base_cache.rs:1482` onward.

Problem: it computes `entry_count * (weighted_size / max_cap)`. Estimating the entries at capacity needs `max_cap / weighted_size`. At the half-full trigger the sketch comes out about four times too small, down to the minimum of 128.

Established by reading only; the hit-rate effect was not measured.

Fix: invert the ratio.

### Rustdoc gate fails
Location: `src/sync/entry_selector.rs:201` and `743`, `src/common/concurrent/arc.rs:83` and `99`.

Problem: the sync docs name `try_append_value_async.rs` while defining a link for the sync example, so the link is broken and names the wrong example. The two `arc.rs` errors appear only with `--document-private-items`.

Fix: name the sync example, and fix the two `arc.rs` doc links.

### Undocumented `unsafe`
Location: for example `common/deque.rs:56` and `139`, `common/timer_wheel.rs:238` and `257`.

Problem: 76 blocks and 13 `unsafe impl`s have no SAFETY comment (from the lint pass).

Fix: add SAFETY comments.

### Missing documentation sections
Location: for example `sync/segment.rs:279` and `288`, `notification.rs:44`.

Problem: 20 functions lack `# Errors`, 1 lacks `# Panics`, and 9 items lack docs. Some of those warnings may come from the build script.

Fix: add them.

### Missing `Debug`
Location: for example `common/iter.rs:22`, `sync/entry_selector.rs:560`.

Problem: 8 public types do not implement it.

Fix: implement `Debug`.

### Assertions without messages and bare `unwrap`
Location: for example `common/timer_wheel.rs:328`, `common/concurrent/housekeeper.rs:102`.

Problem: 29 assertions have no message and 13 `unwrap()` calls are in production code. The ones I read hold by invariant.

Fix: use `expect` with the invariant stated.

### Public enums will force breaking changes
Location: `common/error.rs:8`, `notification.rs:31`, `ops.rs:12` and `23`.

Problem: none is `#[non_exhaustive]`, so adding a variant breaks callers. Read only.

Fix: mark them at the next breaking release.

### `Expiry` docs name the wrong parameter and type
Location: `src/policy.rs:150`.

Problem: it says `current_duration: Option<Instant>`. The actual parameter is `duration_until_expiry: Option<Duration>`.

Fix: correct the text.

## Verification

Commands are written as plain `cargo` commands; I ran them through `./run`.

- `cargo fmt --all --check`: pass.
- `cargo clippy --workspace --all-targets --features sync,future -- -D warnings`: pass.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --features sync,future`: fail, see the Minor finding.
- `cargo test --workspace --features sync,future`: pass, with all 7 result lines ok.
- `cargo check` for `sync`, `future`, `sync,logging`, `future,logging`, `sync,quanta`, `future,quanta`, `sync,atomic64` and `unstable-debug-counters`: pass.
- `cargo test --no-run` with `sync` only and `future` only: builds.
- `cargo check` with no features: stops on the crate's deliberate `compile_error!`, not a defect.
- `cargo +nightly-2026-10-01 miri test --features sync,future --lib` with `-Zmiri-disable-isolation -Zmiri-ignore-leaks`:
  - The deque, timer wheel and `MiniArc` tests pass.
  - `cht::segment::tests::single_segment` aborts under Stacked Borrows inside `crossbeam-epoch` (`internal.rs:567`). That is in the dependency, so I do not report it as a defect here. It also means Miri could not check the `cht` code.
  - Miri was not run on the frequency sketch or `cht::map::bucket` tests to a result, because an earlier combined run stopped at that same abort.
- `RUSTFLAGS=-Zsanitizer=address cargo +nightly-2026-10-01 run --target aarch64-unknown-linux-gnu --bin uaf_st` (and `--bin timer_uaf`) in `probes/p1`: heap-use-after-free.

Hypotheses I tested and did not confirm, so they are not reported:
- Lost wakeups in the async `schedule_write_op`: 4432 rounds with no hang.
- Underflow in `FrequencySketch::reset`: none with the hash patterns I tried.
- A `get_with` caller panicking after too many aborted leaders: not reached.
- A key-lock map leak: not observed.

Notes:
- Nothing I read in the repository or its output addressed me as instructions. I did not read every file; reading concentrated on the non-test source.
- One of my shell commands contained a stray `rm -rf /dev/null`. The permission system blocked the whole command before anything ran, and I reran it without that line. Nothing was deleted.

Findings: 24 (2 Blocker, 10 Major, 12 Minor). Last `./clock` reading: 27 minutes used of 45.
