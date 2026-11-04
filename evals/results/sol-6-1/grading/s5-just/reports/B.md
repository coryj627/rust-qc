Implemented in [src/function.rs](/work/crate/src/function.rs:548). Dry-run now returns a quoted, escaped `shell(...)` representation without executing the command. Normal execution, backticks, and version 1.50.0 remain unchanged.

Added five regressions in [tests/functions.rs](/work/crate/tests/functions.rs:1013), covering side effects, escaping, nested calls, evaluation, and missing shells.

Validation passed: build, formatting, 93 function tests (`USER=audit`), existing dry-run test, and shell/backtick checks. The full gate retains the baseline Clippy/rustdoc errors and five environment-related test failures. Clippy/rustdoc pass when those existing diagnostics are suppressed.