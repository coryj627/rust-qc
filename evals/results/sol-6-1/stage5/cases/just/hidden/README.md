# Held-back oracle and neighboring cases

Bug: `just-6`; repository: `casey_just.git`; supplied fix commit: `0beb79f49def4245757cddf15d52e485332ecb49`.

`case_just_6_upstream.rs` holds the complete, verbatim bodies and attributes of `tests/functions.rs::dry_run_does_not_execute_shell_function` (fixed source line 1641) and `tests/functions.rs::dry_run_shell_function_output_is_escaped` (line 1655). Extraction adds only `use super::*;`. `apply_hidden.py` copies those functions into a separately registered integration module so baseline and fixed can use the same test text, without replacing their existing tests or changing production. Assertions, exact expected stderr, default empty stdout, and success status remain intact. The snapshots' own Test helpers both assert status and output and perform existing parse round trips. `provenance.json` records hashes and the supplied provenance; no Git metadata is present to independently authenticate the commit.

`case_just_6_neighbors.rs` contains nine authored neighbors. They follow only these task requirements:

| Case | Specification basis |
|---|---|
| dry_run_shell_does_not_change_files | “never executes commands”: check a real side effect, beyond an error exit. |
| dry_run_shell_in_assignment_evaluation | “--dry-run never executes”: shell calls in evaluated assignments are also covered. |
| dry_run_quotes_empty_backslash_control_and_unicode_arguments | “properly quoted and escaped arguments” and “otherwise stable”: empty and escaped values must retain meaning. |
| dry_run_quotes_command_argument | The command itself is also a shell argument and needs quoting/escaping. |
| dry_run_nested_shell_calls_are_not_executed | “never executes commands”: inner shell calls must also be suppressed. |
| dry_run_does_not_launch_configured_shell | Unevaluated shell calls cannot need a shell executable. This test disables the harness's default `--shell bash` override. |
| ordinary_shell_execution_preserves_side_effects_arguments_and_trimming | “Preserve actual execution outside dry-run” and stable existing output handling. |
| dry_run_backtick_behavior_is_preserved | “existing backtick behavior”: dry-run retains backticks and suppresses their effects. |
| ordinary_backtick_execution_is_preserved | “existing backtick behavior”: normal execution and trimming continue. |

All tests use the repository's existing dependencies and harness. No test dependency is added. Linux/Bash is the validated environment. These are not new feature requirements, and graders should accept semantically equivalent neighboring representations as explained in rubric.md. The two original upstream tests remain unchanged and exact.

Keep this directory, scoring.md, rubric.md, logs, the fixed tree, and authoring scratch/control directories inaccessible to the treatment agent. Giving only case.md plus an isolated baseline copy is required; a directory name alone is not an access boundary.
