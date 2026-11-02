# Testing

**TEST-1. Unit tests live in an inline `#[cfg(test)] mod tests`** at the bottom of the file they test, with `use super::*;`. memchr, jiff, and fd all follow this; jiff has test modules in 35 source files.

**TEST-2. Integration tests live under `tests/`** and exercise only the public API or the built binary. Group them by feature area in separate files (ripgrep: `binary.rs`, `json.rs`, `multiline.rs`, `regression.rs`). Share helpers through a `tests/util.rs` or `tests/common/mod.rs` module, not by copying.

**TEST-3. Regression tests reference the issue and cover the bug class.** A bug fix adds a test that fails without the fix, named after the issue or placed in a regression file with a link (ripgrep's `tests/regression.rs` holds over 100, named after issue numbers such as `r16`). Do not stop at the exact reported input: add the neighboring cases that the same root cause breaks (more elements, a different order, a combination with other inputs), because a second symptom of the same defect is common.

**TEST-4. Test behavior, not implementation.** Assert observable results and properties. Do not write tests that restate a constant, duplicate the function's own logic to compute the expected value, or mirror its branches one to one.

**TEST-5. Check invariants broadly, without new dependencies first.** When you can state a property (no panic on any input, round trips, equivalence with a naive implementation), the first tool is a plain test: enumerate every input up to a small length over a relevant alphabet, or run a seeded loop of random inputs against a naive reference. That catches most edge cases with no new crates. Reach for `quickcheck` or `proptest` only when the crate already depends on one, or when failures are hard to minimize by hand. memchr checks its SIMD routines against a naive byte loop this way.

**TEST-6. Test names describe the scenario and outcome**: `parse_rejects_empty_unit`, not `test1` or `test_parse`.

**TEST-7. Edge cases are explicit tests**: empty input, one element, maximum sizes, boundaries, non-UTF-8 paths and bytes, and every documented error and panic (`#[should_panic(expected = "...")]`).

**TEST-8. Doc examples are tests.** `cargo test` runs them; keep them meaningful with assertions rather than bare calls.

**TEST-9. Readable assertions.** One behavior per test. `assert_eq!(actual, expected)` with values a reader can verify. For large text output, use a helper that prints a readable diff (ripgrep's `eqnice!` macro), or snapshot testing (`insta`) for stable rendered output, as dua-cli does with `tests/snapshots`.

**TEST-10. Test utilities are feature-gated** if a library exposes them to downstream crates (`feature = "test-util"`), never always-on public API.

**TEST-11. Mockable I/O.** Code that touches the filesystem, clock, network, or environment takes those through a parameter or trait so tests run without real resources and in parallel. Use `tempfile` for tests that must touch disk.

**TEST-12. Cover feature combinations.** Test with default features, with `--no-default-features`, and with `--all-features` (memchr's CI runs all three). For many features use `cargo hack --feature-powerset`.
