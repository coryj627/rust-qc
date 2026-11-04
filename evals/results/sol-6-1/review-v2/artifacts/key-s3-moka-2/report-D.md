Found 11 findings: 2 High, 6 Medium and 3 Low. The original test suites pass, but added regression tests expose seven behavioral defects. Strict documentation builds also fail.

## High

### 1. Borrowed-key fallible entry lookup returns an entry whose key panics

[src/sync/cache.rs:1422](/work/crate/src/sync/cache.rs:1422), [src/common/entry.rs:59](/work/crate/src/common/entry.rs:59).

On a cache hit, `entry_by_ref(...).or_try_insert_with(...)` should return an `Entry` containing its key. The lookup instead hardcodes `need_key = false`. Calling `key()`, or formatting the entry with `Debug`, then panics with `Bug: Key is None`.

Executed regressions reproduce this in both `sync::Cache` and `SegmentedCache`. The corresponding asynchronous test passes. The existing doctest checks the key after insertion but omits that assertion on the subsequent hit.

forward `need_key` and test both insertion and hit paths.

### 2. Clearing expiration can leave a dangling timer handle

[src/common/timer_wheel.rs:493](/work/crate/src/common/timer_wheel.rs:493).

When a scheduled entry’s expiration becomes `None`, advancing the timer wheel should remove the timer and clear its stored handle. The `None` branch instead drops the popped node without clearing the handle in `DeqNodes`.

An executed unit regression schedules an entry, clears expiration through a read performed by `or_insert_with_if`, advances the mocked clock, and runs maintenance. It confirms that expiration is cleared while `timer_node()` remains set.

Source inspection shows subsequent removal or rescheduling can pass this freed node to pointer-dereferencing timer operations. The regression stops at checking the stale handle; it does not exercise that subsequent dereference.

clear the handle before dropping the node and handle this path as a descheduling event.

## Medium

### 3. Asynchronous entry computations can lose updates

[src/future/value_initializer.rs:350](/work/crate/src/future/value_initializer.rs:350), also line 488.

The documented entry-computation contract serializes calls on the same key. Instead, `set_waiter_value` removes the waiter and releases its lock before the resulting cache operation completes.

An executed deterministic regression starts a counter at zero and performs three concurrent `and_upsert_with` increments. An asynchronous eviction listener waits during the first replacement. The other two computations both read `1`, leaving the final value at `2` instead of `3`.

retain the computation guard through completion of the cache operation, preserving cancellation cleanup.

### 4. Returning `None` from expiry update can leave the replacement expired

[src/sync/base_cache.rs:689](/work/crate/src/sync/base_cache.rs:689), [src/future/base_cache.rs:819](/work/crate/src/future/base_cache.rs:819).

`Expiry::expire_after_update` documents that returning `None` clears expiration. For an already-expired deadline, `checked_duration_since` produces `None`. When the callback also returns `None`, the equality check skips clearing the stored deadline.

Executed sync and future regressions configure a 20 ms creation expiry, wait 40 ms, and replace the value with an update callback explicitly returning `None`. An immediate `get` returns `None` instead of the newly inserted value.

distinguish an elapsed deadline from absent expiration and ensure an explicit `None` clears the stored deadline.

### 5. Removal returns values already invalidated by `invalidate_all`

[src/sync/cache.rs:1620](/work/crate/src/sync/cache.rs:1620), [src/future/cache.rs:1964](/work/crate/src/future/cache.rs:1964).

The invalidation contract guarantees that retrieval methods do not return invalidated entries. Removal clones the raw table value without applying the validity checks used by `get`.

Executed tests in both implementations show:

- After inserting `1` and calling `invalidate_all`, `remove` returns `Some(1)`.
- An `and_compute_with` closure receives `None`, returns `Op::Remove`, and receives `CompResult::Removed` containing that invalidated value. The documented result for an absent entry is `StillNone`.

apply visibility checks to returned values while still removing the underlying entry and completing maintenance.

### 6. Out-of-order write logs permanently distort weighted size

[src/sync/base_cache.rs:1460](/work/crate/src/sync/base_cache.rs:1460), [src/future/base_cache.rs:1611](/work/crate/src/future/base_cache.rs:1611).

After completed writes and maintenance, weighted size should converge to the stored entries’ weight. Applying each update’s old/new delta with saturating subtraction loses information when logs arrive out of order.

Executed sync and future regressions pause one update inside an expiry callback, allowing a later update to be processed first. The sole stored value weighs `3`, but maintenance reports `4`. Removing that entry leaves an empty cache reporting weight `1`.

This creates persistent accounting errors that affect capacity decisions.

track the weight actually applied to policy state per entry and reconcile updates by generation.

### 7. Generation wrap prevents maintenance from removing entries

[src/common/concurrent/entry_info.rs:97](/work/crate/src/common/concurrent/entry_info.rs:97).

`entry_gen` wraps as a `u16`, but `set_policy_gen` rejects numerically smaller generations even when they are newer after wrapping. The entry consequently remains marked dirty, and maintenance skips it.

Executed sync and future tests perform maintenance after every replacement, then call `invalidate_all` and repeatedly run maintenance. After 65,534, 65,535 and 65,536 replacements, remaining entry counts are `[0, 1, 1]`; all should be zero. Reads correctly hide the retained entry.

Ordinary frequently updated keys can therefore retain invalidated values and consume capacity indefinitely without another explicit removal.

use consistent modular generation ordering and add wrap-boundary tests.

### 8. Strict documentation builds fail

Locations:

- [src/sync/entry_selector.rs:201](/work/crate/src/sync/entry_selector.rs:201), also line 743.
- [src/common/concurrent/arc.rs:83](/work/crate/src/common/concurrent/arc.rs:83), also line 99.

Executed rustdoc checks fail on two unresolved `try_append_value_async.rs` links, an unresolved `ptr::eq` link, and a bare URL. Public-only documentation fails on the first two; documenting private items exposes all four.

These are pre-existing source defects under Rust 1.99.1, rather than an unavailable-tool failure.

use the defined synchronous example link, qualify `std::ptr::eq`, and wrap the URL in angle brackets.

## Low

### 9. Public API documentation has gaps

Locations: [src/future/entry_selector.rs:242](/work/crate/src/future/entry_selector.rs:242), also line 897; [src/sync/segment.rs:279](/work/crate/src/sync/segment.rs:279).

Advisory Clippy reports 23 missing-item documentation warnings within `src/`. Most consequentially, both `and_try_compute_if_nobody_else` methods lack documentation explaining when computation is skipped and what result callers receive.

document contention behavior, returned variants and error propagation. Twenty additional missing-`# Errors` advisories largely concern APIs already describing errors in prose.

### 10. Unsafe ownership and synchronization arguments are incompletely documented

Locations: [src/common/concurrent/arc.rs:60](/work/crate/src/common/concurrent/arc.rs:60), [src/common/concurrent/arc.rs:115](/work/crate/src/common/concurrent/arc.rs:115), [src/cht/segment.rs:173](/work/crate/src/cht/segment.rs:173).

Advisory Clippy flags 77 unsafe blocks and 13 unsafe implementations. Some have nearby rationale, so these counts are warnings rather than 90 independently demonstrated defects. Confirmed gaps include raw Arc dereferences and zero-initialized segment construction without their validity arguments.

record allocation lifetime, initialization and synchronization invariants at the relevant operations. These omissions make future pointer-management changes harder to review reliably.

### 11. Eight public types lack `Debug`

Locations include [src/future/builder.rs:58](/work/crate/src/future/builder.rs:58), [src/sync/builder.rs:50](/work/crate/src/sync/builder.rs:50), [src/common/iter.rs:22](/work/crate/src/common/iter.rs:22).

Advisory Clippy and source inspection identify eight public builder, selector and iterator types without `Debug`, limiting their use in caller diagnostics.

provide implementations that omit callback internals and avoid unnecessary generic bounds.

## Verification

Checks ran offline with `RUSTUP_AUTO_INSTALL=0`, Rust/Cargo 1.99.1, on `aarch64-unknown-linux-gnu`.

From `/work/crate`:

- `cargo test --locked --features sync,future` — passed: 150 unit tests, 12 integration tests and 67 doctests; seven ignored.
- `cargo fmt --all --check` — passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — passed.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --all-features` — failed with the four documentation errors above.
- `cargo test --workspace --all-features` — passed with the same test totals.
- `cargo check --locked --features sync` and `cargo check --locked --features future` — both passed.
- `RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps --features sync,future` — failed with the two public documentation errors.

The [tooling]’s audit script and heuristic scan completed. Arc, deque and timer-wheel tests passed under `nightly-2026-10-01` Miri, both normally and with `MIRIFLAGS=-Zmiri-tree-borrows`: ten tests per configuration.

Only `/work/tmp/review` was edited: [public regression tests](/work/tmp/review/tests/review_regressions.rs) were added, and a [timer regression module](/work/tmp/review/src/sync/cache.rs:5134) was appended. No production fixes were made. Scratch formatting and strict Clippy passed.

From that scratch copy:

- `cargo test --locked --features sync,future --test review_regressions` — 13 failures reproducing findings and one passing async control.
- `cargo test --locked --features sync,future --lib cleared_expiry_releases_timer_handle` — failed, confirming the stale timer handle.

Total: 11 findings. Coverage was targeted rather than exhaustive; full-suite Miri, Loom, Kani and feature-powerset testing were not run. MSRV and cross-target checks were unavailable with the installed toolchains and targets. No dedicated security assessment was performed. Last clock reading: 13.0615 of 45 minutes used.