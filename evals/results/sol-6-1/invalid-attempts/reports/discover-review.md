Found one Blocker defect. Callers can receive a panic or an incorrect page count for valid inputs. The implementation was not changed.

## Blocker

### Page calculation overflows before division

Location: [src/lib.rs:8](/work/crate/src/lib.rs:8)

Problem: `items + page_size - 1` can overflow even when the correct page count fits in `usize`. For example, `pages(usize::MAX, 2)`:

- With overflow checks enabled, it panics.
- With overflow checks disabled, it returns `Some(0)` instead of `Some(usize::MAX / 2 + 1)`.

Both behaviors were reproduced against the existing source. Callers could consequently skip required pages or allocate insufficient storage.

Fix: retain the zero-size guard and use `items.div_ceil(page_size)` for the calculation.

## Verification

- PASS: `cargo fmt --all --check`
- FAIL: `cargo clippy --workspace --all-targets --all-features -- -D warnings` — two arithmetic lints at the affected calculation.
- PASS: `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --all-features`
- PASS: `cargo test --workspace --all-features` — no tests exist.
- PASS: separate behavior probes confirmed the panic and incorrect result with overflow checks enabled and disabled, respectively.