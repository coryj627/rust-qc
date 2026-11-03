Implemented in [/work/crate](/work/crate).

- Private endpoints preserve valid spans across the full `usize` range.
- Typed errors distinguish reversed spans, bounds failures, and invalid UTF-8 boundaries.
- Added documentation and runnable examples.
- No dependencies added.

Verification passed: rustfmt, Clippy with warnings denied, rustdoc with warnings denied, 8 integration tests, and 12 doctests.