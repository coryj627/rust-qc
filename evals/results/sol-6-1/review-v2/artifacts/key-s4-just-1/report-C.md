Confirmed **11 findings: 1 High, 8 Medium, 2 Low**. The binary builds, and most existing tests pass. Focused local tests demonstrate behavior changes during formatting, incorrect evaluation/output, and a panic through the public Rust API. **No supplied source files were edited.**

## High

### F1. Invalid datetime formats panic instead of returning an error

Locations: [src/function.rs:246](/work/crate/src/function.rs:246), [src/function.rs:250](/work/crate/src/function.rs:250), [src/config.rs:140](/work/crate/src/config.rs:140).

Expected: invalid formats produce a normal diagnostic and `Err` from `just::run`. Actual: `datetime('%Q')`, `datetime_utc('%')`, and `--timestamp-format '%Q'` panic while converting Chrono’s formatted value to a string. CLI invocations exit 101; library callers encounter an unexpected unwind.

Evidence: CLI probes reproduced all three cases. An external Rust test calling `just::run` also panicked. Fix: validate format items and handle formatting errors explicitly.

## Medium

### F2. Formatting removes `eager` and changes execution

[src/assignment.rs:16](/work/crate/src/assignment.rs:16).

Expected: `--fmt` preserves assignment behavior. Actual: it rewrites `eager x := …` as `x := …`. With `set lazy`, an otherwise unused eager assignment subsequently stops running.

Evidence: a scratch fixture’s assignment created a marker before formatting and stopped creating it afterward. Fix: serialize `eager`, and test execution equivalence across formatting.

### F3. Formatting removes privacy from modules and aliases

Locations: [src/item.rs:59](/work/crate/src/item.rs:59), [src/alias.rs:41](/work/crate/src/alias.rs:41).

Expected: `[private]` survives formatting. Actual: both serializers omit it. Previously hidden modules and aliases appear in `--list` afterward.

Evidence: before/after listing tests confirmed both cases. Fix: preserve privacy attributes in both serializers. The existing dump-idempotence helper does not detect this semantic loss.

### F4. User-defined functions discard evaluation context

[src/evaluator.rs:226](/work/crate/src/evaluator.rs:226).

Expected: functions referencing module assignments observe their overridden, already evaluated values. Actual: each call starts with a fresh scope, empty overrides, and reset context.

Evidence:

- With `x=override`, direct `x` printed `override`, while `f() := x` printed `original`.
- An assignment backtick ran again for each function call, producing `x xx xxx`.
- A function backtick missed an exported module variable.
- Inside a dependency, direct `is_dependency()` returned `true`; the function wrapper returned `false`.

Ordinary callers get inconsistent values and repeated commands. Fix: retain module bindings, overrides, exports, and dependency context while keeping function parameters isolated. This feature is documented as unstable.

### F5. Modules sharing a source file receive the wrong identity

[src/compiler.rs:19](/work/crate/src/compiler.rs:19).

Expected: `mod foo 'common.just'` and `mod bar 'common.just'` have distinct module identities. Actual: the compiler caches a context-bearing AST solely by file path.

Evidence: invoking `foo::hello`, whose body prints `module_path()`, printed `bar`. Invoking both recipes executed only once; separate module overrides also collapsed.

distinguish module instances when caching ASTs and assigning recipe/variable identities; source-text caching can remain shared.

### F6. Multiline format strings can lose a leading newline

Locations: [src/parser.rs:939](/work/crate/src/parser.rs:939), [src/evaluator.rs:385](/work/crate/src/evaluator.rs:385).

Expected: adding an empty interpolation leaves a string’s value unchanged. Actual: an indented format string without interpolations is unindented during parsing and again during evaluation.

Evidence: otherwise equivalent fixtures evaluated to `"hello\n"` without interpolation and `"\nhello\n"` with `{{''}}`. This changes generated text. Fix: unindent format strings exactly once.

### F7. Shell evaluation output changes exclamation marks

[src/justfile.rs:323](/work/crate/src/justfile.rs:323).

Expected: shell-formatted assignments reproduce the original values when evaluated. Actual: `x := 'hello!'` produces `x="hello\!"`, which preserves the added backslash in ordinary shell execution.

Evidence: evaluating that output with both `sh -c` and `bash -c` yielded `hello\!`. Fix: use shell quoting that preserves values, and test by evaluating the output rather than comparing its text alone.

### F8. Dotenv values overwrite recipe-specific environment attributes

[src/recipe.rs:368](/work/crate/src/recipe.rs:368).

Expected: a recipe-specific `[env('FOO', 'recipe')]` supplies a consistent value to the recipe. Actual: command setup applies dotenv/module exports after the attribute, overwriting it.

Evidence: with `.env` containing `FOO=dotenv`, a parameter-default backtick observed `recipe`, while the command body observed `dotenv`. The script execution path has the same ordering.

apply recipe environment attributes after common environment exports in both execution paths.

### F9. Submodule aliases are omitted or attributed to the wrong recipe

Locations: [src/subcommand.rs:558](/work/crate/src/subcommand.rs:558), [src/alias.rs:52](/work/crate/src/alias.rs:52).

Expected: an alias to `child::task` retains that identity and displays the target’s parameters. Actual: output uses only the target’s final name.

Evidence: `alias shortcut := child::task` disappeared from `--list`. Adding an unrelated root recipe named `task` made the alias appear attached to it, with its incorrect zero-argument signature. Usage output also printed the unqualified target.

retain qualified recipe identity when grouping and displaying aliases, including aliases targeting another module.

## Low

### F10. Clippy fails on the supplied toolchain

Locations: [src/alias.rs:17](/work/crate/src/alias.rs:17), [src/invocation_parser.rs:333](/work/crate/src/invocation_parser.rs:333), plus lines 404 and 648.

Expected: configured lint checks pass. Actual: four denied assertion-style diagnostics stop Clippy. This blocks contributor checks on Rust 1.99.1; it does not prevent the binary build.

use assertion forms accepted by the configured lints.

### F11. Lexer documentation fails strict rustdoc checks

Locations: [src/lexer.rs:807](/work/crate/src/lexer.rs:807), plus lines 824, 851 and 852.

Expected: lexer patterns render literally. Actual: rustdoc interprets brackets as links or footnotes, producing four errors with warnings denied.

place the patterns in code spans.

## Verification and reproduction

Commands below were executed locally, offline, with `RUSTUP_AUTO_INSTALL=0`.

From `/work/crate`:

- `cargo build --locked --bin just` — **passed**, including the final rerun.
- `cargo test --locked` — **513 unit tests passed; 1,240 integration tests passed, 5 failed, 8 ignored**.
- `cargo fmt --all --check` — **passed**.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — **failed**, F10.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --all-features` — **failed**, F11.
- [tooling] audit/advisory and heuristic scans — executed; candidates were checked against source and project conventions.

The five baseline test failures were environment-sensitive: missing `USER` and runtime-directory configuration, permission semantics, and two non-Unicode directory-creation failures. A local mode-0400 file remained writable in this sandbox.

This adjusted command **passed 513 unit and 1,242 integration tests**, with 8 ignored and 3 explicitly filtered:

```sh
USER=review XDG_RUNTIME_DIR=/work/tmp/review/runtime \
cargo test --locked --workspace --all-features -- \
  --skip format::write_error \
  --skip non_unicode::warn_for_non_unicode_invocation_directory \
  --skip non_unicode::warn_for_non_unicode_justfile_path
```

Reproducers recreate fixtures under `/work/tmp/review`:

```sh
python3 /work/tmp/review/probe.py       # F1, F4
python3 /work/tmp/review/roundtrip.py   # F2, F3, F7
python3 /work/tmp/review/additional.py  # F1, F5, F6, F8
python3 /work/tmp/review/alias.py       # F3, F9
```

External public-API tests:

```sh
cd /work/tmp/review/api
cargo test --locked --offline
```

Result: **4 passed, 1 failed**, reproducing F1. Scratch-crate formatting, Clippy and documentation checks passed.

`cargo +nightly-2026-10-01 miri test --locked --lib unindent` — **4 passed**. This limited run does not cover signal handling or other operating-system interactions.

Windows/macOS execution, cross-target builds, MSRV verification and full Miri testing were not performed. Dedicated security testing, exploitability analysis and exploit development were outside scope and were not pursued.

**Last clock reading: 7.7047 minutes used of 15.0.**