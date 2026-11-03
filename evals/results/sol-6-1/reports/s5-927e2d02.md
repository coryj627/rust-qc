Fixed [src/index_map.rs](/work/crate/src/index_map.rs:1268): shrinking now rebuilds hash indices using the existing `retain` logic, preserving lookup reachability, retained values, length, and iteration order. Public API unchanged.

Added regression coverage for wrapping collision chains, empty through full maps, and zero, intermediate, equal, and larger truncation lengths. Confirmed it fails before the fix.

Verification passed:

- All 23 IndexMap tests, also under Miri.
- `cargo test --offline`.
- `cargo check --offline --lib --no-default-features`.
- CI feature tests and warnings-denied rustdoc.
- Stable formatting check; nightly doc-comment formatting was unavailable.

Strict Clippy still reports the same pre-existing diagnostics, with none added.