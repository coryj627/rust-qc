Found **11 issues**: ten reproduced behavior defects and one toolchain-specific lint failure. `/work/crate` was unchanged; regression scripts, fixtures, and logs are under `/work/tmp`.

All behavior findings were reproduced through the CLI. Root-cause explanations below come from source inspection.

1. **Formatting multiline module documentation produces invalid syntax** — [src/item.rs:68](/work/crate/src/item.rs:68).

   A valid module declaration using `[doc("first\nsecond")]` should remain valid after formatting. `just --fmt` instead writes a comment containing `first`, followed by a bare `second` line. Formatting exits successfully, but the next `just --list` exits 1 with a parse error. Ordinary formatting can therefore leave the justfile unusable.

2. **Formatting removes `eager`, silently changing execution** — [src/assignment.rs:16](/work/crate/src/assignment.rs:16).

   With `set unstable`, `set lazy`, and `eager check := ` followed by the backtick command `exit 9`, invocation initially exits 9. After `just --fmt`, `eager` is gone and invocation exits 0. The existing `lazy::eager_assignments_are_evaluated` test establishes the intended behavior. Formatting can silently disable initialization or validation assignments.

3. **Two module names loading the same file share an execution identity** — [src/compiler.rs:19](/work/crate/src/compiler.rs:19).

   Given `mod a 'shared.just'` and `mod b 'shared.just'`, each module should retain its own path, scope, and recipe identity. With a shared recipe printing `module_path()`, `just a::show b::show` prints only `b`, exiting 0; expected output is `a` followed by `b`. The compiler caches the parsed source solely by file path, retaining one module context. A requested recipe is silently skipped; separate module overrides also interfere.

4. **References can ignore redefined constants** — [src/evaluator.rs:411](/work/crate/src/evaluator.rs:411).

   Constant redefinition is explicitly supported by `tests/constants.rs`. However, with `A := HEX` and `HEX := 'custom'`, a recipe printing both produces `0123456789abcdef custom`, rather than `custom custom`. Similarly, `just --evaluate z` for `z := HEX` returns the predefined constant despite the redefinition.

   Lookup finds the inherited prelude binding before resolving the local assignment. Results consequently depend on assignment-name ordering and which variables are selected for evaluation.

5. **`lazy` is controlled by the root module instead of each module’s setting** — [src/justfile.rs:187](/work/crate/src/justfile.rs:187).

   The README states that each module uses its own settings. A child containing `set lazy`, an unused backtick assignment running `exit 9`, and an unrelated recipe should run that recipe successfully. Without root-level `set lazy`, invocation exits 9.

   The reverse also fails: root-level `set lazy` suppresses the unused assignment in a child using the default eager behavior. This can unexpectedly execute expensive assignments or skip intended initialization.

6. **Unused assertions are evaluated during settings analysis** — [src/evaluator.rs:37](/work/crate/src/evaluator.rs:37).

   `set lazy` should skip unused assignments, and selective `--evaluate` already has tests establishing that unrelated assignments are skipped. Nevertheless, an unused assignment containing `assert('a' == 'b', 'unused')` prevents an unrelated recipe from running. It also makes `just --evaluate good` fail when `good := 'ok'`; even `--list` fails.

   Settings analysis evaluates every assignment and immediately propagates assertion errors, before runtime selection can exclude unused variables.

7. **Formatting removes private visibility from modules and aliases** — [src/item.rs:59](/work/crate/src/item.rs:59), [src/alias.rs:42](/work/crate/src/alias.rs:42).

   `[private]` should continue hiding the same items after formatting. Executed before/after `--list` comparisons showed that `--fmt` removes the attribute from both module declarations and aliases. Previously hidden items then appear in listings.

   The existing round-trip helper at [tests/test.rs:335](/work/crate/tests/test.rs:335) checks dump idempotence, which cannot detect a modifier lost during the first dump.

8. **Shell evaluation output changes values containing `!`** — [src/justfile.rs:323](/work/crate/src/justfile.rs:323).

   For `foo := 'hello!world'`, `just --evaluate --evaluate-format shell` emits `foo="hello\!world"`. Sourcing that output in both `sh` and Bash produces `hello\!world`, with an extra backslash.

   Shell-format output should preserve the original value. The existing test at [tests/evaluate.rs:264](/work/crate/tests/evaluate.rs:264) asserts the incorrect escaped text without checking its interpreted value.

9. **User-defined functions discard overrides and reevaluate module assignments** — [src/evaluator.rs:226](/work/crate/src/evaluator.rs:226), [src/evaluator.rs:241](/work/crate/src/evaluator.rs:241).

   Functions may reference assignments in their module. With `x := 'original'` and `f() := x`, invoking `just x=override show` prints `override original`; both references should observe `override`.

   A second regression referenced one backtick-backed assignment directly and through two function calls. Its counter file recorded three executions instead of one. Each function starts with a fresh root scope and empty override map, producing inconsistent values and repeating external commands. These findings require the documented unstable function feature.

10. **Invalid date formats panic instead of returning an error** — [src/function.rs:245](/work/crate/src/function.rs:245), [src/function.rs:249](/work/crate/src/function.rs:249), [src/config.rs:137](/work/crate/src/config.rs:137).

    `datetime('%Q')`, `datetime_utc('%')`, and `--timestamp --timestamp-format '%Q'` all exit 101 with an uncaught panic: “a Display implementation returned an error unexpectedly.” An invalid format should produce an ordinary diagnostic. A routine format typo instead crashes execution.

11. **The configured Clippy check fails on Rust 1.99.1** — [src/alias.rs:17](/work/crate/src/alias.rs:17).

    `cargo clippy --locked --all --all-targets --all-features -- --deny warnings` fails on `clippy::manual_assert_eq`, enabled by the package’s denied `pedantic` group.

    Repeating the command with `--allow clippy::manual_assert_eq` reveals additional denied `assert_is_empty` diagnostics at [src/invocation_parser.rs:333](/work/crate/src/invocation_parser.rs:333), lines 404 and 648, and `tests/choose.rs:243`. This is a confirmed failure with the supplied Clippy version; older toolchains were not tested.

The executed project checks used Rust **1.99.1**, Clippy **0.1.99**, and aarch64 Linux. Cargo build/test/lint/doc runs used `RUSTUP_AUTO_INSTALL=0`, offline dependency resolution, and two build jobs.

| Command | Outcome |
|---|---|
| `cargo build --locked --bin just` | Passed. |
| `cargo test --locked` | 513 unit tests passed; integration: 1,240 passed, 5 failed, 8 ignored. |
| `cargo test --locked --workspace` | Same five integration failures; workspace execution stopped there. |
| `cargo clippy --locked --all --all-targets --all-features -- --deny warnings` | Failed as described above. |
| Same Clippy command plus `--allow clippy::manual_assert_eq` | Failed on additional `assert_is_empty` diagnostics. |
| `cargo fmt --all -- --check` | Passed. |
| `RUSTDOCFLAGS='-D warnings' cargo doc --locked --no-deps --all-features` | Passed. |
| `cargo test --locked --test integration parallel:: -- --ignored` | All three selected parallel tests passed. |
| `python3 /work/tmp/review_regressions.py` | Three control methods passed; 15 regression methods produced 18 failure records, including subtests. |

The five initial failures were environment-dependent:

- `directories::runtime_directory`: `XDG_RUNTIME_DIR` was unset.
- `functions::env_var_functions_unix`: the test unwraps the unset `USER` variable.
- `format::write_error`: an independent probe confirmed that this sandbox permits writing a mode-0400 file.
- Both `non_unicode::warn_for_non_unicode_*` tests: directory creation itself fails with OS error 84, before invoking `just`.

This adjusted run passed **513 unit tests and 1,242 integration tests**, with eight ignored and three explicitly skipped:

```sh
USER=audit XDG_RUNTIME_DIR=/work/tmp/runtime cargo test --locked -- \
  --skip format::write_error \
  --skip non_unicode::warn_for_non_unicode_invocation_directory \
  --skip non_unicode::warn_for_non_unicode_justfile_path
```

The reproducible harness is [review_regressions.py](/work/tmp/review_regressions.py), with detailed results in [review-regressions.log](/work/tmp/review-regressions.log). Its fixtures retain command arguments, exit codes, stdout, and stderr.

**Finding count: 11.** Coverage focused on `crate/src/`, using documentation and existing tests as the contract. Other platforms, minimum-Rust compatibility, release-profile builds, Miri, book/site checks, and remaining ignored signal tests were not run. No dedicated security testing was performed. Last `/work/clock` reading: **8.97335 minutes used of 45.0**.