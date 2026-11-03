Fixed [`src/index_map.rs`](/work/crate/src/index_map.rs): `truncate` now rebuilds the hash index after shortening, preserving lookup reachability, retained values, iteration order, and length. Public API unchanged.

Added regression coverage for collisions, wraparound, empty through full maps, and zero/intermediate/equal/larger truncation lengths. Confirmed it fails before the fix and passes afterward.

Verification passed:

- Index-map tests: 23 passed.
- Full suite: 187 unit, 9 integration, 177 documentation tests passed.
- `cargo check --offline --lib --no-default-features`
- Formatting check.