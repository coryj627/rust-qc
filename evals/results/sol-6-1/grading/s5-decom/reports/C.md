Implemented in `alacritty_terminal` only.

- DECOM set/reset homes using the updated origin and clears pending wrap.
- DECSC/DECRC saves and restores origin mode per screen without changing the saved cursor position.
- Added seven escape-sequence regression tests covering non-default margins, both saved states, wrapping, and neighboring behavior. Updated the crate changelog.

Validation: terminal-only locked/offline tests passed with default features (180 tests) and without default features (135 tests). All seven regressions also passed under Miri.

Limitations: strict Clippy and rustdoc checks retain their pre-existing failures. Nightly rustfmt is unavailable; stable formatting checks show no new differences in changed files.