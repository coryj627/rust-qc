Found **5 issues: 4 runtime correctness/error-handling issues and 1 documentation issue**. The supplied source was unchanged; targeted tests were added only to `/work/tmp/moka-review`.

1. **High — Async computations release serialization before applying the operation.**  
   [src/future/value_initializer.rs:350](/work/crate/src/future/value_initializer.rs:350), also line 488.  
   The documented behavior serializes same-key compute/upsert calls. Instead, `set_waiter_value` consumes the guard and removes the waiter before the subsequent insertion or removal. Another computation can therefore read the previous value and overwrite an update.  
   **Executed evidence:** `future_upserts_must_serialize_through_write` pauses the first insertion in its weigher and starts another increment. Both computations read `0`; the final value is `1`, expected `2`. Callers using these APIs for counters or accumulated state can lose updates. Keep serialization active through operation application.

2. **Medium — Borrowed synchronous fallible entry lookup loses the key on a cache hit.**  
   [src/sync/cache.rs:1422](/work/crate/src/sync/cache.rs:1422).  
   The hit path passes `false` instead of `need_key`. Consequently, `entry_by_ref(...).or_try_insert_with(...)` returns an `Entry` with no key when the value already exists. `Entry::key()` should return the cached key; it instead panics with `Bug: Key is None`.  
   **Executed evidence:** both `sync_borrowed_try_entry_retains_key_on_hit` and `segmented_borrowed_try_entry_retains_key_on_hit` fail. The equivalent async control passes. Ordinary callers can panic while inspecting a successfully returned entry. Forward `need_key` on the hit path.

3. **Medium — Returning `None` from an expiry update can leave the replacement expired.**  
   [src/sync/base_cache.rs:696](/work/crate/src/sync/base_cache.rs:696) and [src/future/base_cache.rs:826](/work/crate/src/future/base_cache.rs:826).  
   An elapsed deadline becomes `current_duration = None` through `checked_duration_since`. If `expire_after_update` also returns `None`, the equality check skips clearing the existing deadline. The documented result is no expiration; the actual replacement remains immediately expired.  
   **Executed evidence:** synchronous and async tests create an immediately expired entry, then replace it with an update callback returning `None`. Both subsequent lookups return `None`, expected `Some(2)`. Callers cannot reliably revive entries this way. Distinguish elapsed expiration from absent expiration.

4. **Medium — A weigher panic leaves synchronous initialization permanently stale for that key.**  
   [src/sync/value_initializer.rs:153](/work/crate/src/sync/value_initializer.rs:153).  
   Cleanup handles a panic from `init`, but insertion and value cloning occur outside that handler. If insertion’s weigher panics, the registered waiter remains `Computing` after its lock is released. Subsequent initialization attempts panic instead of retrying.  
   **Executed evidence:** `sync_get_with_can_retry_after_weigher_panic` runs the first call in a worker thread whose weigher panics once. After joining that failed worker, a retry panics with `Got unexpected state Computing`; the equivalent async control succeeds. This is an error-recovery weakness—recovery from weigher panics is not explicitly documented. Use cleanup that covers the complete waiter lifetime.

5. **Low — Two synchronous API example links are broken.**  
   [src/sync/entry_selector.rs:201](/work/crate/src/sync/entry_selector.rs:201) and [src/sync/entry_selector.rs:743](/work/crate/src/sync/entry_selector.rs:743).  
   Both references name `try_append_value_async.rs`, while their link definitions name `try_append_value_sync.rs`. Readers should receive a working synchronous example link; instead, rustdoc reports unresolved links.  
   **Executed evidence:** documentation generation with warnings denied exits `101` at these locations.

Executed project checks, from `/work/crate` with `RUSTUP_AUTO_INSTALL=0`:

```sh
cargo test --locked --features sync,future
cargo test --locked --offline --all-features
cargo clippy --locked --offline --features sync,future --all-targets -- -D warnings
cargo check --locked --offline --no-default-features --features sync
cargo check --locked --offline --no-default-features --features future
cargo fmt --all -- --check
RUSTDOCFLAGS='-D warnings' cargo doc --locked --offline --features sync,future --no-deps
```

Both test configurations passed **150 unit tests, 12 integration tests, and 67 doctests**, with five unit tests and two doctests ignored. Clippy, feature-isolated checks, and formatting passed. Strict documentation generation failed as described above.

Reproduce the targeted failures using the retained [test file](/work/tmp/moka-review/tests/review_regressions.rs):

```sh
cd /work/tmp/moka-review
RUSTUP_AUTO_INSTALL=0 cargo test --locked --offline \
  --features sync,future --test review_regressions -- --nocapture
```

Result: **6 failed assertions across the 4 runtime findings; 2 controls passed**. Output is retained in [review-regressions.log](/work/tmp/review-regressions.log).

Source inspection focused on public entry APIs, initialization, expiration, invalidation, iteration, and maintenance coordination. This was not exhaustive concurrency verification. Miri, Loom, other architectures, and MSRV execution were unrun; no checks were blocked. Dedicated security testing and related inquiries were outside scope and were not pursued.

Last `/work/clock` reading: **5.551233333333333 minutes used of 15.0**.