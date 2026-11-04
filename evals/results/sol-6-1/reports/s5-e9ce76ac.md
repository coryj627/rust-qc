Implemented in [term/mod.rs](/work/crate/alacritty_terminal/src/term/mod.rs):

- DECOM set/reset homes using the new origin and clears pending wrap.
- DECSC/DECRC saves and restores origin mode per screen buffer without changing public interfaces.
- Added escape-sequence regressions for non-default margins, both saved origin states, wrapping, screen switches, reset, and neighboring behavior.

The requested locked/offline terminal-only test command passed: **133 unit tests, 44 reference tests, and 1 doctest**.

Changed-file stable rustfmt passed. Full formatting verification remains limited by unavailable nightly rustfmt and pre-existing formatting differences elsewhere.