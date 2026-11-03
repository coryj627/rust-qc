Fixed [shell()](/work/crate/src/function.rs:548): dry-run now returns `shell("command", "arg", …)` with quoted, escaped arguments without executing commands. Normal execution, backticks, and version 1.50.0 remain unchanged.

Added three [regression tests](/work/crate/tests/functions.rs:1013) covering side effects, assignments, defaults, interpolation, shebang recipes, and argument escaping.

Validation passed: offline build, 91 function tests, existing dry-run test, 45 backtick tests, 9 shell tests, and formatting. One backtick test was already ignored. The function suite required setting `USER`, which the sandbox omitted.