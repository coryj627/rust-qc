Implemented DECOM homing and origin-mode save/restore in `alacritty_terminal`, preserving public interfaces and existing cursor state handling.

Added seven escape-sequence regression tests covering non-default margins, pending wrap, both saved origin states, alternate screens, and reset behavior. Updated the crate changelog.

Validation used locked, offline, terminal-only commands:

- All-features suite: 180 tests passed.
- Without default features: 135 tests passed.
- New regressions under Miri: all seven passed.

Strict Clippy and rustdoc checks retain their baseline failures, with no new diagnostics. Project-format verification remains limited because nightly rustfmt is unavailable; stable rustfmt reports existing formatting differences.