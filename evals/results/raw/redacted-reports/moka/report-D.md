# Audit: moka 0.12.11

Verdict: the crate builds and its own test suite passes, but it has real defects in the paths those tests skip. Safe code can reach a use-after-free through the timer wheel. Ordinary concurrent insert and invalidate calls can permanently corrupt a cache's accounting, until it retains nothing. The async compute guarantee does not hold, and `invalidate_entries_if` leaves entries behind. 

Unless a finding says "read only", I reproduced it through the public API from a throwaway crate in `probes/p1/src/bin/`, run with `./run`. `sync` and `future` share most of this logic, so I give both locations where they apply. `crate/` was not changed. Internal probes used a copy at `probes/copy`.

## High

**Use-after-free: timer node freed while its pointer stays in the entry**
`src/common/timer_wheel.rs:493-526`. The stale pointer is dereferenced at `timer_wheel.rs:276-279`, reached through `sync/base_cache.rs:1753-1755` and `1767-1768` (`future/base_cache.rs` has the same paths).

`TimerEventsIter::next` pops a node and only handles `if let Some(t) = expiration_time`. If an `Expiry` has cleared the expiration by then, the box is dropped without `unset_timer_node_in_deq_nodes`, so `DeqNodes.timer_node` dangles. The next reschedule or removal then reads and writes freed memory. Clearing can happen when `expire_after_read` returns `None` during maintenance, when the `ReadOp` is discarded because the channel is full, or when a read is not recorded. The comment at `sync/base_cache.rs:342-345` says these cases are okay; they are not.

memory corruption or a crash from safe code.

- `uaf_st.rs` is single-threaded: `expire_after_read` returns `None`, `get_with_if` makes a read that is not recorded, then sleep, `run_pending_tasks` and `invalidate`.
  - AddressSanitizer reports heap-use-after-free.
  - The release build panics with `index out of bounds: the len is 5 but the index is 175` at `timer_wheel.rs:279`, which is garbage read from the freed node.
- `timer_uaf.rs` does it with two threads and `get`, holding maintenance open through a blocking eviction listener. AddressSanitizer reports heap-use-after-free and the release build dies with `Bus error`.

**Removed entries get admitted as permanent phantoms, and the cache stops retaining anything**
`src/sync/base_cache.rs:1458-1475` and `1785-1787`, `src/future/base_cache.rs:1609` onward, with op sending at `sync/cache.rs:1487-1489` and `1601-1637`.

`insert` changes the map and sends its `WriteOp` as two separate steps, and so does `invalidate`, so a `Remove` can be applied before the matching `Upsert`. `handle_upsert` then sees "not admitted" and admits an entry that is no longer in the map. The phantom is counted and stays in the deques. Eviction can never remove it, because `remove_if` returns `None` and `skip_updated_entry_*` only rotates it to the back.

What a user sees, with 4 threads doing `insert`/`invalidate` on 16 keys (`phantom.rs`, `phantom2.rs`, `phantom_fut.rs`):
- A capacity-1000 cache ends with `entry_count=970 weighted=1000 actual=0`.
- Inserting 1000 fresh keys afterwards retains 0. The control without churn retains 1000.
- An unbounded cache grows without limit: `entry_count` went 14343, 29260, 45682, 62094 over 4 rounds, with 0 real entries.
- A 100 ms TTL does not clear the phantoms.
- `future::Cache` behaves the same: `entry_count=954`, 0 of 1000 retained.

## Medium

**Async compute loses updates despite the documented serialization**
`src/future/value_initializer.rs:350` and `488` (documented at `future/entry_selector.rs:163`).

`set_waiter_value(ReadyNone)` consumes the guard, which releases the lock and removes the waiter before the returned `Op` is applied. A second compute can therefore read the old value.

What a user sees (`compute_atomic.rs`, 8 tasks, 800000 increments):
- `and_compute_with` gave 799916.
- `and_upsert_with` gave 799881.
- `sync` gave exactly 800000.

**`invalidate_entries_if` skips the entry at every batch boundary**
`src/sync/base_cache.rs:2153` and `2170`, `src/future/base_cache.rs:2351` and `2368`.

`iter.peek()` on a `Peekable<&mut Deque>` advances the deque's own cursor, and the peeked element is discarded. Once the predicate finishes it is removed, so skipped entries become readable again.

What a user sees (`inv.rs`, `fut_misc.rs`): after a match-everything predicate on 2000 entries, survivors are `[384, 769, 1154, 1539, 1924]`, and `contains_key` returns true for them. Sync and future agree.

**A second predicate starts from a stale scan cursor**
`src/sync/base_cache.rs:2170` and `2184-2186`.

the cursor is reset only when no predicates remain. If a new predicate is waiting when the current one finishes, its scan continues from the old position. The trailing `peek()` after exhaustion has also already restarted the iterator and consumed the head.

What a user sees (`inv.rs`): with a first predicate in progress and a second match-everything predicate, survivors are `[0, 385, 770, 1155]` of 1200, and key 0 is the head skipped here. A predicate that finishes part way through the deque would also skip everything before the cursor; that wider case is from reading only, because my probe for it (`inv2.rs`) did not reach it.

**After about 65536 updates of one key, the entry stays dirty almost permanently**
`src/common/concurrent/entry_info.rs:97`.

`current >= value` rejects the wrapped generation. With `policy_gen` at 65535 and `entry_gen` wrapped to 1, the update never lands, so `is_dirty()` stays true. Dirty entries are skipped by predicate scans, TTL eviction and size eviction.

What a user sees (`gen.rs`, 65540 updates):
- A match-all predicate leaves `key1=Some(65539)` readable.
- After TTL expiry `entry_count` stays 1; the 60000-update control reaches 0.
- In an LRU cache of capacity 1, the hot key stays present while 190 newer keys are evicted.

**Updating an entry whose per-entry expiry has passed leaves the new value expired**
`src/sync/base_cache.rs:689-706`, `src/future/base_cache.rs:826`.

`checked_duration_since` gives `None` for a past expiration, so `duration != current_duration` is false when the `Expiry` returns `None`. That is what the default `expire_after_update` returns in this case, and it is also how a user asks for no expiration. The old past timestamp is kept.

What a user sees (`expiry.rs`): after the old expiry passes, `insert(1, 2)` followed by `get(&1)` returns `None`, both with the default update method and with one that explicitly returns `None`.

**`weighted_size` drifts under concurrent updates with changing weights**
`src/sync/base_cache.rs:525` and `650`, `src/future/base_cache.rs:525` and `780`.

the retryable `on_modify` closure reads `old_weight` from the shared `EntryInfo`, then overwrites it. Failed or racing attempts mutate that value, so the deltas no longer add up.

What a user sees (`weight.rs`): two keys finishing at weight 5 each report `weighted_size` of 87427, 75939 and 3176 where 10 is expected. A bounded cache would then evict against a wrong size.

**`get_with` panics when it waits on `try_get_with` with error type `()`**
`src/sync/value_initializer.rs:348` with `sync/cache.rs:1053`, `src/future/value_initializer.rs:592` with `future/cache.rs:1597`.

`get_with` uses `TypeId::of::<()>()` as its waiter key, the same key `try_get_with::<_, ()>` uses. The error reaches `InitErr(_) => unreachable!()`.

What a user sees (`waiter.rs`, `fut_misc.rs`): `internal error: entered unreachable code` in both caches, on a caller that did nothing wrong.

**Panic outside the guarded closure poisons a key in `sync::Cache`**
`src/sync/value_initializer.rs:137`, `153` and `278`.

only `init` and `f` are wrapped in `catch_unwind`. A panic in `insert` (a weigher, for example) or in `get` leaves the waiter in the map. The async version has a drop guard for this.

What a user sees (`waiter.rs`, `misc.rs`):
- After one panicking weigher call, every later `get_with` on that key panics with `Got unexpected state Computing`, even after `invalidate`.
- After a panic in `and_upsert_with`, the next `and_upsert_with` on that key had still not returned after 3 s. From the code it spins in the retry loop.

**`remove` returns values that were already invalidated or expired**
`src/sync/cache.rs:1601-1623`, `src/future/cache.rs:1959`.

no validity check is made. The `invalidate_all` docs (`sync/cache.rs:1646-1648`) promise retrieval methods will not return such entries. From the code, `and_compute_with` returning `Op::Remove` goes through the same path and can report `Removed` with a stale value; my compute probe got `StillNone` because maintenance ran first.

What a user sees (`misc.rs`, `weight.rs`):
- After `invalidate_all`: `get=None`, `remove=Some(1)`.
- With a match-all predicate registered: `get=None`, `remove=Some(1)`.
- After TTL expiry: `remove=Some(1)`.

**Removal listener reports the wrong cause for per-entry expiry**
`src/sync/base_cache.rs:2306-2333` and `2336-2355`, `src/future/base_cache.rs:2513` and `2554`.

only TTL and TTI are checked. `OldEntryInfo` does not record the expiration time.

What a user sees (`expiry.rs`): an entry past its `Expiry` time is reported as `Replaced` on overwrite and `Explicit` on invalidate. `RemovalCause::Expired` is documented for exactly this case.

## Low

**Expiry set by an unrecorded read never gets a timer**
`src/sync/base_cache.rs:342-345`.

when the `ReadOp` is lost and the entry has no timer yet, nothing ever evicts it.

What a user sees (`notimer.rs`): `get=None`, `entry_count=1`, indefinitely.

**`or_insert_with_if` reports no replacement when it replaced a value**
`src/sync/cache.rs:1050`, `src/future/cache.rs:1594`.

What a user sees (`misc.rs`): `value=2 fresh=true old_replaced=false`, which contradicts `Entry::is_old_value_replaced`. Established by running the sync path; the future location is from reading.

**`invalidate_all` misses entries from the same clock tick**
`src/sync/base_cache.rs:2463`, `src/future/base_cache.rs:2694`.

`entry_ts < va` is strict, but the docs promise "before or at".

What a user sees (mock clock test in `probes/copy`): `get` returns `Some(1)`.

**One `run_pending_tasks` processes only one predicate batch**
`src/sync/base_cache.rs:1272-1274` and `2181-2189`.

`invalidate_entries` never increments `eviction_count`, so the loop stops after one batch.

What a user sees (`misc.rs`): `entry_count=1616` of 2000 after one call, while the same call after `invalidate_all` gives 0.

**Frequency sketch size formula looks inverted for weighted caches**
`src/sync/base_cache.rs:1348`, and the same formula at `src/future/base_cache.rs:1482` onward.

it computes `entry_count * (weighted_size / max_cap)`. Estimating the entries at capacity needs `max_cap / weighted_size`. At the half-full trigger the sketch comes out about four times too small, down to the minimum of 128.

Established by reading only; the hit-rate effect was not measured.

`src/sync/entry_selector.rs:201` and `743`, `src/common/concurrent/arc.rs:83` and `99`.

the sync docs name `try_append_value_async.rs` while defining a link for the sync example, so the link is broken and names the wrong example. The two `arc.rs` errors appear only with `--document-private-items`.

**Undocumented `unsafe`**
for example `common/deque.rs:56` and `139`, `common/timer_wheel.rs:238` and `257`.

76 blocks and 13 `unsafe impl`s have no SAFETY comment (from the lint pass).

**Missing documentation sections**
for example `sync/segment.rs:279` and `288`, `notification.rs:44`.

20 functions lack `# Errors`, 1 lacks `# Panics`, and 9 items lack docs. Some of those warnings may come from the build script.

**Missing `Debug`**
for example `common/iter.rs:22`, `sync/entry_selector.rs:560`.

8 public types do not implement it.

**Assertions without messages and bare `unwrap`**
for example `common/timer_wheel.rs:328`, `common/concurrent/housekeeper.rs:102`.

29 assertions have no message and 13 `unwrap()` calls are in production code. The ones I read hold by invariant.

**Public enums will force breaking changes**
`common/error.rs:8`, `notification.rs:31`, `ops.rs:12` and `23`.

none is `#[non_exhaustive]`, so adding a variant breaks callers. Read only.

**`Expiry` docs name the wrong parameter and type**
`src/policy.rs:150`.

it says `current_duration: Option<Instant>`. The actual parameter is `duration_until_expiry: Option<Duration>`.
