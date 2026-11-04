Found one caller-visible defect. The implementation was not changed.

## Blocker

### Page-count arithmetic overflows on valid inputs

Location: [src/lib.rs:8](/work/crate/src/lib.rs:8)

The intermediate addition in `(items + page_size - 1)` can overflow even when the correct page count fits in `usize`.

Confirmed through the public API:

- `pages(usize::MAX, 2)` panics with overflow checks enabled. In a default release build, it returns `Some(0)` instead of `Some(usize::MAX / 2 + 1)`.
- `pages(usize::MAX, 1)` also panics with overflow checks enabled, although the correct result is simply `Some(usize::MAX)`.

Callers can therefore encounter a panic or silently receive an incorrect page count, depending on build settings.

Fix: retain the zero-size guard and use overflow-safe ceiling division. Add regression coverage around `usize::MAX`.

## Verification

- `cargo fmt --all --check`: passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: failed on `manual_checked_ops` and `manual_div_ceil`.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --all-features`: passed.
- `cargo test --workspace --all-features`: passed; the library contains no tests.

An external probe checked 10,201 small-input cases successfully and reproduced the overflow in both debug and release builds.