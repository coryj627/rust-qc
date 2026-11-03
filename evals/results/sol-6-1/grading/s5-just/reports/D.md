Implemented in [src/function.rs](/work/crate/src/function.rs:548): dry-run returns `shell("command", "arg", …)` with quoted, escaped arguments without executing the command. Normal execution, backticks, and version 1.50.0 remain unchanged.

Added four regressions in [tests/functions.rs](/work/crate/tests/functions.rs:1013), confirmed failing before the fix and passing afterward.

Validation passed: offline build, 92 function tests, 9 shell tests, 45 backtick tests (1 ignored), 5 dry-run tests, and formatting. Set `USER=audit` for the existing environment-dependent test.