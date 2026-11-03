Implemented in [term/mod.rs](/work/crate/alacritty_terminal/src/term/mod.rs):

- DECOM set/reset home using the new origin and clear pending wrap.
- DECSC/DECRC save and restore origin mode per screen buffer, alongside existing cursor state.

Added [escape-sequence regressions](/work/crate/alacritty_terminal/tests/origin_mode.rs) covering non-default margins, last-column wrap, both saved origin states, rendition, character sets, alternate screens, and reset.

Validation passed: **177 tests** using `RUSTUP_AUTO_INSTALL=0 TMPDIR=/work/tmp cargo +1.99.1 test --locked --offline -p alacritty_terminal`. All four new tests fail against the original implementation.

Stable rustfmt checks passed; nightly-only formatting options were unavailable. Only `alacritty_terminal` was built and tested.