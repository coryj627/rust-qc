Implemented `byte_span` in [src/lib.rs](/work/crate/src/lib.rs).

- Private fields enforce ordered spans.
- Typed errors distinguish reversal, bounds failures, and invalid UTF-8 boundaries.
- Documented all methods with runnable examples.
- Added no dependencies.

Verification passed: rustfmt, Clippy with warnings denied, rustdoc with warnings denied, 10 unit tests, and 12 doctests. Tests cover `usize::MAX`, UTF-8 boundaries, error precedence, and intersections.