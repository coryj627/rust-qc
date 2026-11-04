Found **six issues: five correctness defects and one documentation defect**. All five correctness defects were reproduced in debug and release builds.

1. **Async compute operations can lose updates.**  
   [src/future/value_initializer.rs:350](/work/crate/src/future/value_initializer.rs:350).

   `and_compute_with` promises to serialize operations on the same key. However, `set_waiter_value` consumes the guard and releases the key’s compute lock before the requested insertion or removal finishes.

   **Executed evidence:** `future_compute_keeps_lock_until_write_finishes` pauses an async eviction listener while three compute operations increment the same counter. Expected results are `1, 2, 3`, leaving `3` cached. Actual results are `1, 2, 2`, leaving `2`. Callers implementing counters or other read-modify-write operations can lose changes. Existing compute tests delay the computation itself but do not exercise this subsequent write suspension.

2. **A normal borrowed-entry cache hit produces an entry whose `key()` panics.**  
   [src/sync/cache.rs:1422](/work/crate/src/sync/cache.rs:1422).

   `get_or_try_insert_with_hash_by_ref_and_fun` passes `false` to `get_with_hash`, ignoring its `need_key` argument. Consequently, `entry_by_ref(...).or_try_insert_with(...)` returns an existing value without its key.

   **Executed evidence:** Both `sync_borrowed_try_entry_hit_has_key` and `segmented_borrowed_try_entry_hit_has_key` insert `"key" → 42`, retrieve the existing entry, verify its value, and then panic at [src/common/entry.rs:59](/work/crate/src/common/entry.rs:59): `Bug: Key is None`. Expected behavior is to return `"key"`. Formatting the entry with `Debug` also calls `key()` and can panic, based on source inspection. The equivalent future-cache test passes.

3. **Reordered weighted updates corrupt capacity accounting and prevent valid admissions.**  
   Locations: [src/sync/base_cache.rs:1460](/work/crate/src/sync/base_cache.rs:1460), [src/future/base_cache.rs:1611](/work/crate/src/future/base_cache.rs:1611).

   Concurrent updates can enqueue their maintenance records in a different order from their changes to the hash table. Applying each record’s old/new weight delta with saturating subtraction loses accounting information when subtraction underflows.

   **Executed evidence:** The sync and future `out_of_order_weight_updates` tests start with weight `1`, pause the update to `2` inside its expiry callback, and complete the update to `3` first. After both writers finish and maintenance runs, the sole cached value is `3`, but `weighted_size()` is `4`.

   A further synchronous test with capacity `3` observes the fitting value being evicted, leaving an empty cache reporting weight `1`. Inserting another weight-`3` value is then rejected. This creates persistent unnecessary cache misses after concurrent activity has ended.

4. **Returning `None` from an update-expiry callback fails to clear an elapsed deadline.**  
   Locations: [src/sync/base_cache.rs:696](/work/crate/src/sync/base_cache.rs:696), [src/future/base_cache.rs:826](/work/crate/src/future/base_cache.rs:826).

   The `Expiry::expire_after_update` documentation says returning `None` clears expiration. For an elapsed deadline, `checked_duration_since` yields `None`; the equality optimization then treats the callback’s returned `None` as unchanged and retains the expired timestamp.

   **Executed evidence:** Both `clear_elapsed_expiry_on_update` tests create an entry with a 10 ms lifetime, wait 30 ms, then replace it. The update callback is confirmed to execute and returns `None`. Expected: `get` returns the replacement value `2`. Actual: `get` returns `None`. An ordinary refresh can therefore complete successfully while leaving its replacement unreadable.

5. **Generation-counter wrap prevents subsequent policy eviction.**  
   [src/common/concurrent/entry_info.rs:97](/work/crate/src/common/concurrent/entry_info.rs:97).

   Entry generations deliberately wrap as `u16`, but `current >= value` prevents the policy generation from following the wrap from `65535` to `0`. The entry remains marked dirty, so maintenance skips it.

   **Executed evidence:** Both `lru_after_generation_wrap` tests perform 65,536 writes to one key in a capacity-one LRU cache, running maintenance after every write. After inserting another key and running maintenance, the older key still returns `Some(65535)` instead of being evicted. A frequently updated entry can become persistently ineligible for policy eviction once writes stop. No concurrent scheduling is needed.

6. **Two broken documentation links fail warning-free documentation builds.**  
   Locations: [src/sync/entry_selector.rs:201](/work/crate/src/sync/entry_selector.rs:201), [src/sync/entry_selector.rs:743](/work/crate/src/sync/entry_selector.rs:743).

   Both examples reference undefined `try_append_value_async.rs` links, while defining links named `try_append_value_sync.rs`.

   **Executed evidence:** Ordinary stable and docs.rs-style nightly documentation builds emit two broken-link warnings. With `RUSTDOCFLAGS='-D warnings'`, documentation generation exits `101`. Readers cannot follow these example links.

All regression cases are in [review_regressions.rs](/work/tmp/review-crate/tests/review_regressions.rs). The original `/work/crate` is unchanged; comparison confirms this test file is the only addition in the scratch copy.

Checks ran on **Rust/Cargo 1.99.1, aarch64 Linux**, with `RUSTUP_AUTO_INSTALL=0`. Commands below ran from `/work/crate`, except the regression commands, which ran from `/work/tmp/review-crate`.

| Command | Outcome |
|---|---|
| `cargo test --locked --features sync,future` | Passed: 150 unit, 12 integration, 67 doctests; seven ignored |
| `cargo test --locked --all-features` | Passed: same counts |
| `cargo test --locked --features sync` | Passed: 99 unit, six integration, 37 doctests |
| `cargo test --locked --features future` | Passed: 92 unit, six integration, 30 doctests |
| `cargo clippy --locked --lib --tests --all-features --all-targets -- -D warnings` | Passed |
| `cargo fmt --all -- --check` | Passed |
| `cargo doc --locked --no-deps --features sync,future` | Passed with the two warnings above |
| `RUSTDOCFLAGS='-D warnings' cargo doc --locked --no-deps --features sync,future` | Failed on those links |
| `RUSTDOCFLAGS='--cfg docsrs' cargo +nightly-2026-10-01 doc --locked --no-deps --features sync,future` | Passed with those warnings |
| `cargo check --locked --no-default-features` | Intentional compile error requiring `sync` or `future`; not a finding |
| `cargo test --locked --features sync,future --test review_regressions -- --nocapture --test-threads=1` | One passing control, ten failing cases covering the five correctness defects |
| `CARGO_BUILD_JOBS=2 cargo test --release --locked --features sync,future --test review_regressions -- --nocapture` | Same one passing control and ten failures |

The existing enabled test suites had no failures. Coverage was selective source inspection and local regression testing; ignored tests, the complete release-mode baseline, Miri, Loom, other architectures, and MSRV compatibility were not tested. Rust 1.70 was unavailable among installed toolchains. No dedicated security assessment was performed.

**Finding count: 6. Last clock reading: 9.4011 minutes used of 45.**