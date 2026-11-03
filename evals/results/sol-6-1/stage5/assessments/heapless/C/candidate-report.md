Fixed [src/index_map.rs](/work/crate/src/index_map.rs:1268): `truncate` now rebuilds indices through existing retention logic, preserving lookup reachability, retained values, order, and length. Public API unchanged.

Added collision and wraparound regression coverage across all requested truncation cases, including subsequent updates and reinsertion. Confirmed the regression fails before the fix.

Verification passed:

- IndexMap tests: 23 passed.
- Full suite: 187 unit, 9 integration, 177 doc tests passed.
- Library check with no default features.
- Formatting check.