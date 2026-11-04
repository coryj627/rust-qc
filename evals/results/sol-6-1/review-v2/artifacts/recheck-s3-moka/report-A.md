The review found **9 findings: 2 Blocker, 4 Major, and 3 Minor**. The supplied tests pass, but public API regression probes confirmed five behavioral defects. No production source files were changed; the added tests are in [the scratch harness](/work/tmp/probe/src/lib.rs).

## Blocker

### 1. Async compute operations release their lock before committing the result

Location: [src/future/value_initializer.rs:350](/work/crate/src/future/value_initializer.rs:350).

Expected: Concurrent `and_upsert_with` calls execute serially, as documented in `src/future/entry_selector.rs:335`.

Observed: `set_waiter_value` consumes the guard and releases the per-key lock before the subsequent insert or removal. Another computation can therefore read the previous value before the first computation commits.

Evidence: An executed, deterministic single-thread test used an async eviction listener to pause writes. Two upserts both read `10` and left `11`; serialized execution should read `10`, then `11`, and leave `12`. A separate four-worker test also lost increments.

Consequence: Callers using these APIs for counters or read-modify-write operations can silently lose updates.

Fix: Retain the waiter registration and lock through the completed mutation. Apply the same correction to the analogous `try_compute_if_nobody_else` path and add concurrent regression tests.

### 2. Mixing infallible and unit-error loaders can panic

Locations: [src/sync/value_initializer.rs:348](/work/crate/src/sync/value_initializer.rs:348), [src/future/value_initializer.rs:592](/work/crate/src/future/value_initializer.rs:592).

Expected: `get_with` remains infallible when another caller concurrently uses `try_get_with` on the same key.

Observed: Both `get_with` and `try_get_with::<_, ()>` use `TypeId::of::<()>()` to identify their waiters. If the fallible loader returns `Err(())`, the infallible loader receives that error and reaches `unreachable!()`.

Evidence: Executed sync and async tests both panicked at `src/sync/cache.rs:1053` and `src/future/cache.rs:1597`. A control using a different error type passed.

Consequence: An ordinary loader error can panic another caller whose initializer cannot fail.

Fix: Include the operation kind in waiter identity, or use a private marker type for infallible initialization.

## Major

### 3. Returning `None` does not clear an already-expired deadline

Locations: [src/sync/base_cache.rs:689](/work/crate/src/sync/base_cache.rs:689), [src/future/base_cache.rs:819](/work/crate/src/future/base_cache.rs:819).

Expected: `Expiry::expire_after_update` returning `None` clears expiration, as documented in `src/policy.rs`.

Observed: An elapsed deadline becomes `current_duration = None` because `checked_duration_since` fails. When the callback also returns `None`, the equality check skips clearing the stored deadline.

Evidence: Executed tests inserted an immediately expired value, then replaced it with a value whose expiry callback returned `None`. Both cache variants returned `None` from the next `get`, instead of the replacement. Clearing an unexpired deadline passed its control test.

Consequence: A successful replacement can remain immediately expired and unavailable.

Fix: Distinguish an elapsed deadline from an absent deadline, and ensure a callback returning `None` clears the stored timestamp.

### 4. Removal returns values that have already been invalidated

Locations: [src/sync/cache.rs:1619](/work/crate/src/sync/cache.rs:1619), [src/future/cache.rs:1964](/work/crate/src/future/cache.rs:1964).

Expected: Retrieval respects logical invalidation. The compute API also documents `Op::Remove` on an absent entry as returning `StillNone`.

Observed: Removal clones the physical hash-table value without applying the validity checks used by reads.

Evidence: Executed sync and async tests inserted `42`, called `invalidate_all`, and confirmed `get` returned `None`. Nevertheless, `remove` returned `Some(42)`. In separate compute tests, the closure received `None`, but returning `Op::Remove` produced `Removed(42)` instead of `StillNone`.

Consequence: Callers can receive invalidated data and contradictory compute results before maintenance removes the underlying entry.

Fix: Check logical validity when determining the returned value, while still performing physical removal and maintenance bookkeeping.

### 5. Generation wrap prevents maintenance from cleaning up hot entries

Location: [src/common/concurrent/entry_info.rs:97](/work/crate/src/common/concurrent/entry_info.rs:97).

Expected: The wrapping generation counters continue to distinguish pending updates from fully processed entries.

Observed: `current >= value` rejects the transition from policy generation `65535` to `0`. Consequently, the entry remains marked dirty after its write operations have been processed.

Evidence: Executed sync and async tests performed 65,535, 65,536, and 65,537 writes to one key, then invalidated everything and ran maintenance three times. Remaining entry counts were respectively `0`, `1`, and `1`.

Consequence: Frequently updated entries retain invalidated values and consume capacity after maintenance. Source inspection also shows dirty entries being skipped during eviction.

Fix: Compare generations using wrapping sequence-number ordering, without the ordinary numeric-order rejection. Add wrap-boundary tests.

### 6. Strict documentation builds fail

Locations: [src/sync/entry_selector.rs:201](/work/crate/src/sync/entry_selector.rs:201), [src/sync/entry_selector.rs:743](/work/crate/src/sync/entry_selector.rs:743), [src/common/concurrent/arc.rs:83](/work/crate/src/common/concurrent/arc.rs:83), [src/common/concurrent/arc.rs:99](/work/crate/src/common/concurrent/arc.rs:99).

Expected: Documentation builds successfully with warnings denied.

Observed: Both public entry-selector examples reference an unresolved `try_append_value_async.rs` link. Documenting private items additionally exposes an unresolved `ptr::eq` link and a bare URL.

Evidence: Both strict rustdoc commands listed below failed on Rust 1.99.1.

Consequence: Documentation CI using warnings as errors fails; published API examples contain broken links.

Fix: Reference the defined synchronous example link, qualify `std::ptr::eq`, and mark the URL as a hyperlink.

## Minor

### 7. Unsafe ownership and lifetime invariants lack local justification

Representative locations: [src/common/concurrent/arc.rs:60](/work/crate/src/common/concurrent/arc.rs:60), [src/common/concurrent/arc.rs:115](/work/crate/src/common/concurrent/arc.rs:115), [src/cht/map/bucket.rs:36](/work/crate/src/cht/map/bucket.rs:36).

The advisory pass reported **77 unsafe blocks and 13 unsafe implementations** without recognized safety comments. Reviewed examples leave pointer validity, initialization, reference-count ownership, or thread-safety reasoning implicit.

This makes changes to these internal abstractions harder to validate. Add local explanations of the required invariants. This is a maintainability finding, not a demonstrated memory-safety defect.

### 8. Public API behavior is undocumented

Representative locations: [src/future/entry_selector.rs:242](/work/crate/src/future/entry_selector.rs:242), [src/future/entry_selector.rs:897](/work/crate/src/future/entry_selector.rs:897).

The advisory pass identified **23 undocumented public items**, including items exposed through re-exports and the unstable debug-counter feature. In particular, neither `and_try_compute_if_nobody_else` method explains when it skips computation or what it returns under contention.

Callers must inspect implementation details to use these methods correctly. Document their contention behavior, results, and errors. The pass also reported 20 missing formal `# Errors` sections; several already describe errors in prose.

### 9. Eight public types lack `Debug`

Representative locations: [src/future.rs:58](/work/crate/src/future.rs:58), [src/future/builder.rs:58](/work/crate/src/future/builder.rs:58), [src/common/iter.rs:22](/work/crate/src/common/iter.rs:22).

The advisory pass confirmed missing `Debug` implementations on two builders, four entry selectors, and two iterator types. Callers cannot format these handles for diagnostics or derive `Debug` on enclosing types that contain them.

Add implementations that expose useful state without requiring callback closures themselves to implement `Debug`.

## Verification

Commands ran on **aarch64 Linux, Rust 1.99.1**, with `RUSTUP_AUTO_INSTALL=0`.

From `/work/crate`:

- **PASS:** `cargo test --locked --features sync,future` — 150 unit, 12 integration, and 67 documentation tests passed; 7 tests were ignored.
- **PASS:** `cargo fmt --all --check`
- **PASS:** `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- **PASS:** `cargo test --workspace --all-features` — same pass/ignore counts.
- **FAIL, pre-existing:** `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --all-features` — four documentation errors described above.
- **FAIL, pre-existing:** `RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps --features sync,future` — the two public example links fail.
- **PASS:** `cargo check --locked --no-default-features --features sync`
- **PASS:** `cargo check --locked --no-default-features --features future`

The required audit script and heuristic scan completed:

- `bash /work/skill/scripts/check.sh --audit /work/crate`
- `python3 -I /work/skill/scripts/audit_scan.py /work/crate`

Targeted Miri checks used:

`MIRIFLAGS=-Zmiri-tree-borrows cargo +nightly-2026-10-01 miri test --locked --lib --features sync,future <filter>`

Executed separately with `common::concurrent::arc::tests`, `common::deque::tests`, and `common::timer_wheel::tests`: **all 10 tests passed**.

From `/work/tmp/probe`, the final check script passed formatting, Clippy, and rustdoc. Its `cargo test --workspace --all-features` run produced **5 passing controls and 12 expected regression failures** covering the five behavioral findings. Results are retained in [probe-gate.log](/work/tmp/probe-gate.log).

**Finding count: 9 — 2 Blocker, 4 Major, 3 Minor.** Coverage was concentrated on public cache behavior, expiration, invalidation, loading, concurrent computation, and maintenance boundaries. Full Miri, exhaustive scheduling/feature combinations, Loom, Kani, and configuration-gated compile tests were not run. MSRV and cross-target checks were unavailable with the installed toolchains and targets. No dedicated security assessment was performed.

**Last clock reading: 646.3 seconds used of 2,700 seconds — 10.76875 of 45 minutes.**