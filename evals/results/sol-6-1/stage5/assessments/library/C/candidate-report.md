Implemented `byte_span` in [/work/crate](/work/crate), with no dependencies.

- Private endpoints preserve `start <= end`.
- All requested methods handle the full `usize` range.
- Distinct errors identify reversed spans, bounds failures, and invalid UTF-8 boundaries.
- Rustdoc explains byte units, invariants, and errors with runnable examples.

Verification passed: 13 integration tests, 3 documentation tests, formatting, Clippy with warnings denied, and documentation generation with warnings denied.