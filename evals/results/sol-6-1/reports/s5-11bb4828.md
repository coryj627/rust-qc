Implemented in [src/function.rs](/work/crate/src/function.rs:548): dry-run returns `shell("command", "arg", …)` with quoted, escaped arguments before launching a process. Normal execution and backtick behavior are preserved. Version and dependencies are unchanged.

Added five regression tests covering side effects, interpolation, shebangs, escaping, nested calls, missing shells, and normal execution.

Validation passed: offline locked build, formatting, 93 function tests with `USER=audit`, existing dry-run test, 45 backtick tests, and 9 shell tests.

The full quality gate reproduced the baseline’s existing Clippy/rustdoc errors and five environment-dependent test failures. Clippy passed with only the two pre-existing lint categories exempted.