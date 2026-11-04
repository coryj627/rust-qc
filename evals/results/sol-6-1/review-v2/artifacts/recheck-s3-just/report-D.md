The review found 12 findings: 2 Blockers, 8 Major, and 2 Minor. The binary builds successfully, but local regression tests demonstrated formatting changes that corrupt or change justfiles, incorrect module and function behavior, and unexpected crashes. No production source files were edited.

## Blocker

### 1. Formatting multiline module documentation corrupts the justfile

Location: [src/item.rs:68](/work/crate/src/item.rs:68).

Expected: `--fmt` preserves a valid module’s documentation and produces a valid justfile.

Observed: `[doc("line 1\nline 2")]` followed by `mod foo` becomes:

```text
# line 1
line 2
mod foo
```

Executed evidence: `--list` initially succeeded; `--fmt` returned success and overwrote the scratch fixture; the next `--list` failed with `Unknown start of token '2'`. An ordinary formatting operation therefore leaves a working file unusable.

Fix: preserve the documentation attribute and its escaped string representation. Add a format-and-reparse test for multiline module documentation; the existing formatting test covers only a single-line comment.

### 2. Variable cycles through functions abort the process

Location: [src/assignment_resolver.rs:61](/work/crate/src/assignment_resolver.rs:61).

Expected: circular variable dependencies receive a diagnostic, consistent with direct variable-cycle handling.

Executed input:

```text
set unstable
f() := a
a := f()
```

Observed: `just --evaluate a` overflowed its stack and terminated with SIGABRT. Source inspection shows that resolving a call checks its signature without following the function body’s assignment dependencies.

Consequence: an accidental circular definition terminates the entire process, including an application using `just::run`.

Fix: account for function references during dependency analysis and guard recursive evaluation. Add direct and indirect cycles involving functions to the existing cycle tests.

## Major

### 3. Formatting removes eager evaluation

Location: [src/assignment.rs:16](/work/crate/src/assignment.rs:16).

Expected: formatting preserves the `eager` modifier.

Executed input used `set lazy`, `eager check := ` followed by the backtick command `exit 42`, and an unrelated `show` recipe. Before formatting, `just show` exited 42. After successful `--fmt`, the assignment lacked `eager`, and `just show` printed `OK` and exited 0.

Consequence: formatting can silently disable validation or required setup.

Fix: serialize `assignment.eager`. Test recipe behavior before and after formatting; formatting idempotence alone misses this loss.

### 4. Modules sharing a source file acquire the same identity

Location: [src/compiler.rs:19](/work/crate/src/compiler.rs:19).

Expected: modules declared with distinct names retain distinct module paths and overrides.

Executed fixture:

```text
mod a 'shared.just'
mod b 'shared.just'
```

`shared.just` contained a recipe printing `module_path()`. Running `just a::show b::show` printed only `b`, instead of `a` followed by `b`. With separate overrides, the sole invocation used `b`’s value.

Source inspection confirms that compilation caches context-bearing ASTs solely by file path. Consequently, recipe deduplication also treats these distinct module recipes as identical.

Fix: distinguish module instances from cached source text, including their logical module context. Add a shared-source regression test.

### 5. User-defined functions discard module evaluation state

Location: [src/evaluator.rs:226](/work/crate/src/evaluator.rs:226), [src/evaluator.rs:241](/work/crate/src/evaluator.rs:241).

Expected: a function referencing a module assignment sees that assignment’s evaluated, overridden value.

Executed tests demonstrated:

- With `x := 'original'`, `f() := x`, and `x=override`, direct access printed `override`, while `f()` returned `original`.
- With `x := uuid()`, direct access and two calls to `f()` produced three different values.
- An exported module variable was missing from a function’s `shell()` command.
- Inside a dependency, direct `is_dependency()` returned `true`, while a function wrapping it returned `false`.

Consequence: extracting an expression into a function changes configuration values, repeats evaluation, and loses execution context.

Fix: share the module’s evaluated bindings and overrides through a separate parameter scope, and preserve the relevant execution flags. Existing function tests do not cover these combinations.

### 6. Shell-format evaluation changes values containing exclamation marks

Location: [src/justfile.rs:323](/work/crate/src/justfile.rs:323).

Expected: shell-format assignments reproduce the evaluated values when sourced.

Executed evidence: `foo := 'hello!'` produced `foo="hello\!"`. Sourcing that output with `/bin/sh` yielded the literal value `hello\!`, including the added backslash.

Consequence: callers importing generated configuration receive incorrect values.

Fix: use shell quoting that round-trips values. The existing test at [tests/evaluate.rs:274](/work/crate/tests/evaluate.rs:274) asserts the incorrect text; replace or supplement it with a shell round-trip test.

### 7. Formatting removes alias and module privacy

Locations: [src/alias.rs:42](/work/crate/src/alias.rs:42), [src/item.rs:59](/work/crate/src/item.rs:59).

Expected: `[private]` continues to suppress the annotated alias or module from listings after formatting.

Observed: executed before/after tests showed that `--fmt` removes `[private]` from both constructs. The formerly hidden alias and module then appeared in `--list`.

Consequence: formatting changes the command interface presented to users.

Fix: serialize alias attributes and the module’s privacy flag. Add behavioral format-and-list tests; the current round-trip helper checks repeated dumps rather than preservation of the original behavior.

### 8. Relative search ceilings are ignored

Location: [src/search.rs:196](/work/crate/src/search.rs:196).

Expected: `--ceiling .` prevents searching above the invocation directory, as stated by the option’s help.

Executed evidence: from an empty child directory, `just --ceiling . show` found and ran the parent justfile, printing `PARENT`. Passing the child’s absolute path instead correctly returned `No justfile found`.

Consequence: callers can unexpectedly run a recipe from a parent project.

Fix: resolve and normalize the ceiling against the invocation directory before comparison. Existing ceiling tests supply absolute paths only.

### 9. Invalid datetime formats panic

Locations: [src/function.rs:246](/work/crate/src/function.rs:246), [src/function.rs:250](/work/crate/src/function.rs:250), [src/config.rs:137](/work/crate/src/config.rs:137).

Expected: invalid user-supplied formats produce a normal error with useful context.

Executed evidence: `datetime('%Q')`, `datetime_utc('%')`, and `--timestamp --timestamp-format '%Q'` each panicked with “a Display implementation returned an error unexpectedly” and exited 101.

Consequence: a format typo produces an internal panic instead of a function or option diagnostic.

Fix: validate the format or handle formatting errors explicitly. Add invalid-format boundary cases alongside the existing valid-format tests.

### 10. Cross-module aliases disappear or receive incorrect usage information

Location: [src/subcommand.rs:558](/work/crate/src/subcommand.rs:558).

Expected: public aliases to submodule recipes appear in listings with their actual target’s signature.

Executed evidence: with `alias quick := foo::build`, `--list --alias-style separate` omitted `quick`. Adding an unrelated local `build` made `quick` appear as an alias for that local recipe, with its parameter list.

Source inspection shows aliases grouped by the target’s final name rather than its full identity.

Consequence: users cannot discover an available command or receive incorrect instructions for invoking it.

Fix: retain the full target identity when rendering aliases. Test cross-module aliases both with and without a same-named local recipe.

## Minor

### 11. Clippy fails with the installed toolchain

Locations: [src/alias.rs:17](/work/crate/src/alias.rs:17); [src/invocation_parser.rs:333](/work/crate/src/invocation_parser.rs:333), lines 404 and 648.

Executed evidence: Clippy 0.1.99 rejected four assertions under the crate’s denied pedantic lint configuration: one `manual_assert_eq` and three `assert_is_empty` diagnostics.

Consequence: the strict lint check fails on Rust 1.99.1 despite a successful build.

Fix: update those assertions or explicitly define the supported lint-toolchain policy. Rust 1.85 was unavailable, so its lint outcome was not established.

### 12. Strict private documentation generation fails

Locations: [src/lexer.rs:807](/work/crate/src/lexer.rs:807), lines 824, 851 and 852.

Executed evidence: documentation generation with private items and warnings denied reported one broken intra-doc link and three broken-footnote diagnostics. Regex notation in comments is interpreted as Markdown.

Consequence: the skill’s strict documentation gate cannot complete. Public-only documentation generation passed.

Fix: put the regex notation in code spans or escape its Markdown punctuation.

## Verification

Commands ran from `/work/crate`, using installed tools and prefetched dependencies, with automatic toolchain installation disabled.

- `cargo build --locked --bin just` — passed.
- `cargo test --locked` — 513 unit tests passed; integration results were 1,240 passed, 5 failed, 8 ignored.
- `bash /work/skill/scripts/check.sh --audit /work/crate` — formatting passed; Clippy and strict private documentation failed as reported above; workspace tests reproduced the five baseline failures.
- `RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps` — passed.
- `python3 -I /work/skill/scripts/audit_scan.py /work/crate` — completed. Its 60 print-macro candidates were CLI output, not separate defects.
- `python3 /work/tmp/review_regressions.py` — 18 checks executed: 16 failures supporting the 10 behavioral findings, plus 2 passing controls.

The gate’s underlying checks were:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --all-features
cargo test --workspace --all-features
```

The five baseline test failures were environment-dependent:

- `directories::runtime_directory`: no runtime directory was configured.
- `functions::env_var_functions_unix`: `USER` was unset.
- Both `non_unicode` tests: fixture-directory creation failed with EILSEQ.
- `format::write_error`: the sandbox allowed the write that the test expected permissions to deny.

This adjusted run passed 513 unit tests and 1,242 integration tests, with 8 ignored and 3 filtered out:

```sh
USER=audit XDG_RUNTIME_DIR=/work/tmp cargo test --locked -- \
  --skip format::write_error \
  --skip non_unicode::warn_for_non_unicode_invocation_directory \
  --skip non_unicode::warn_for_non_unicode_justfile_path
```

Miri was attempted with:

```sh
cargo +nightly-2026-10-01 miri test --lib run::tests::run_can_be_called_more_than_once
```

It stopped at isolated `mkdir`. Repeating with `MIRIFLAGS=-Zmiri-disable-isolation` stopped at unsupported `fcntl(F_SETFD)`. These are tool limitations, not demonstrated defects. Advisory lint output also included three undocumented hidden exports and eight reasonless `allow` attributes; no missing `Debug` or undocumented-unsafe diagnostics appeared.

All added harnesses, fixtures and logs are under `/work/tmp`; build artifacts are under `/target`. Rust 1.85, Windows/macOS execution, cross-target checks, the full Miri suite and dedicated security testing were not run.

Finding count: **12 — 2 Blocker, 8 Major, 2 Minor**. Coverage was a targeted correctness and quality review of `crate/src/`, not exhaustive path coverage. Last clock reading: **554.8 seconds used of 2,700 seconds (9.24185 of 45 minutes)**.