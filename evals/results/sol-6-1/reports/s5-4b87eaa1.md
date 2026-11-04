Fixed [`IndexMap::truncate`](/work/crate/src/index_map.rs:1268) by reusing order-preserving retention to rebuild collision probe chains. Public API remains unchanged.

Added regression coverage for retained lookups, values, iteration order, length, wraparound collisions, boundary truncation lengths, and reinsertion. Confirmed it fails before the fix and passes afterward.

Passed:

- `cargo test --offline --lib index_map::tests::` — 23 tests
- `cargo test --offline`
- `cargo check --offline --lib --no-default-features`
- Compatible-feature tests and strict rustdoc checks
- All 23 IndexMap tests under Miri

Clippy reports the same pre-existing failures as baseline, with no new diagnostics. Stable formatting checks pass with nightly-option warnings; nightly rustfmt is unavailable.