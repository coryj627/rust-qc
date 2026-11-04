The review found **6 findings: 3 Medium and 3 Low**. Existing tests pass, but ordinary public-API probes reproduced lost updates, ineffective expiration clearing, and retained expired entries. The supplied source was unchanged; all review harnesses are under `/work/tmp/probes`.

## Medium

### 1. Async computations release their lock before applying the result

`src/future/value_initializer.rs:350`, also `:488`.

Expected: Same-key compute/upsert calls execute serially, including applying their updates.

Actual: `set_waiter_value` removes the waiter and releases its guard before the subsequent insert/remove completes. If that operation awaits a key lock, another computation can read the same old value.

Caller impact: Concurrent counter increments or other read-modify-write operations can lose updates.

Evidence: While an async eviction listener held the key lock, two `and_upsert_with` calls both read `100`, both returned `101`, and left `Some(101)`. Serialized increments should leave `Some(102)`. The reproduction exits 101 on that assertion.

Retain the compute guard through applying the operation, including awaited writes, while preserving cancellation cleanup.

### 2. Returning `None` from an expiry update can leave expiration set

`src/sync/base_cache.rs:689`, `:696`; `src/future/base_cache.rs:819`, `:826`.

Expected: An expiry callback returning `None` clears the entry’s expiration, as documented.

Actual: An elapsed deadline becomes `current_duration == None` through `checked_duration_since`. If the callback also returns `None`, the equality optimization skips clearing the stored deadline.

Caller impact: A replacement value can become immediately unreadable. `get_with` can return a freshly computed value that the following `get` cannot retrieve.

Evidence: A policy immediately expired value `10` and returned `None` for replacement value `20`. Sync/future `insert` and `get_with` all left subsequent reads returning `None`; all four should return `Some(20)`. This reproduction uses no sleeps.

Distinguish an elapsed deadline from absent expiration, and ensure a callback’s `None` clears the stored deadline.

### 3. Dropped read operations can permanently lose expiration timers

`src/sync/base_cache.rs:476`, `:1380`; `src/future/base_cache.rs:470`, `:321`.

Expected: When `expire_after_read` adds expiration to a previously non-expiring entry, maintenance eventually removes it.

Actual: A full read-operation queue discards the operation carrying the timer update. The existing recovery explanation assumes a timer already exists; a newly expiring entry has none.

Caller impact: Expired values remain stored, expiration notifications are omitted, and maintained counts retain those entries.

Evidence: In both implementations, a briefly blocked maintenance listener allowed the read queue to fill. A subsequent read added a 40 ms expiration. After another 1.3 seconds and five maintenance calls:

- Iteration found one live entry.
- `entry_count` remained two.
- Removing the expired key still returned `Some(1)`, confirming retention.

Controls without queue saturation passed: count one, expired key already removed.

Make expiration-changing read operations reliable, or retain a rescheduling flag that maintenance can recover independently of discarded popularity records.

## Low

### 4. Public documentation has gaps and strict rustdoc fails

Locations: `src/sync/entry_selector.rs:201`, `:743`; `src/common/concurrent/arc.rs:83`, `:99`; `src/future/entry_selector.rs:242`, `:897`.

Expected: Documentation builds cleanly and explains public methods’ behavior.

Actual: Strict rustdoc fails on two incorrect example links, unresolved `ptr::eq`, and a bare URL. The advisory pass also found 23 undocumented public items under `src/`, including both `and_try_compute_if_nobody_else` methods; 20 functions lack formal `Errors` sections, although many explain failures in prose.

Caller impact: Broken references and incomplete contracts hinder API use, particularly concurrent compute methods.

Correct the four references and document the exposed methods’ execution, error, and cancellation behavior.

### 5. Eight public types lack `Debug`

Locations include `src/sync/builder.rs:50`, `src/future/builder.rs:58`, `src/common/iter.rs:22`, `src/future.rs:58`, and the four entry-selector structs.

Expected: Public helper types support diagnostic formatting.

Actual: Both builders, both iterators, and four selectors lack `Debug`, confirmed by Clippy.

Caller impact: Callers cannot directly format these objects or derive `Debug` for structures containing them.

Add manual implementations where closures or internal trait objects prevent deriving it.

### 6. Manual concurrency implementations omit their invariants

Locations include `src/future/cache.rs:641`, `src/sync/cache.rs:576`, `src/common/concurrent/arc.rs:60`, `src/common/iter.rs:132`.

Expected: Manual `Send`/`Sync` implementations explain why their bounds, ownership, and synchronization permit cross-thread use.

Actual: Thirteen implementations lack that local justification. Some comments explain why the compiler needs an implementation, without explaining the invariants that maintainers must preserve.

Caller impact: Future changes to bounds or internal synchronization are harder to review correctly.

Document those invariants at each implementation.

## Executed checks

Commands ran from `/work/crate`, offline, with:

```sh
export RUSTUP_AUTO_INSTALL=0 CARGO_NET_OFFLINE=true CARGO_BUILD_JOBS=2
```

Passed:

```sh
cargo test --locked --features sync,future
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --locked --features sync
cargo check --locked --features future
```

Each full test configuration passed **150 unit tests, 12 integration tests, and 67 doctests**, with seven ignored tests.

Failed with the four documentation diagnostics described above:

```sh
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps \
  --document-private-items --all-features
```

Targeted Miri checks passed all ten existing MiniArc, deque, and timer-wheel tests:

```sh
for module in arc deque timer_wheel; do
  MIRIFLAGS=-Zmiri-tree-borrows cargo +nightly-2026-10-01 miri test \
    --locked --features sync,future --lib "common::$module::tests"
done
```

For the MiniArc command, use filter `common::concurrent::arc::tests`.

Reproduce the behavioral findings:

```sh
cargo run --locked --manifest-path /work/tmp/probes/Cargo.toml --bin expiry
cargo run --locked --manifest-path /work/tmp/probes/Cargo.toml --bin compute
cargo run --locked --manifest-path /work/tmp/probes/Cargo.toml --bin timer
cargo run --locked --manifest-path /work/tmp/probes/Cargo.toml --bin timer_async
```

All four exit 101 on the demonstrated incorrect behavior. Both timer controls pass when invoked with `-- --control`.

The `model` binary passed 10,000 mixed operations against a reference map across sync, segmented, and future caches. Scratch harness formatting and strict Clippy also passed. Logs are under `/work/tmp`, including `gate.log`.

## Coverage limits

Source inspection focused on public contracts, initialization, compute serialization, expiration, maintenance, invalidation, iteration, and supporting structures. Cross-target builds, MSRV validation, Loom, trybuild-specific checks, exhaustive feature combinations, and full-crate Miri were unrun. Advisory lint candidates were not treated as independently confirmed defects.

No dedicated security assessment, exploitability analysis, or exploit development was performed. Inquiries requiring dedicated security testing remain outside scope.

Last clock reading: **9.8997 minutes used of 15.0**.