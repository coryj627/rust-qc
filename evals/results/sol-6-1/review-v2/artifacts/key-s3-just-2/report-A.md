Review found 11 issues: 3 High, 6 Medium, and 2 Low. The binary builds, but ordinary regression probes exposed incorrect evaluation, formatting changes that alter behavior, and input-triggered panics.

All behavioral findings below were reproduced through the built CLI. Reachability through the public `just::run()` API was established by source inspection. No crate source files were changed; harnesses and generated justfiles are confined to `/work/tmp`.

## High

### 1. Formatting multiline module documentation corrupts valid source

[src/item.rs:68](/work/crate/src/item.rs:68).

Expected: `--fmt` preserves a valid justfile’s meaning and parseability.

Observed: `[doc("first\nsecond")]` followed by `mod child` becomes:
```text
# first
second
mod child
```
`--fmt` succeeds and overwrites the file, but the next invocation rejects line 2. This can leave an ordinary project unusable after formatting.

Evidence: `test_format_multiline_module_docs_remains_valid` reproduced the successful write followed by a parse failure.

preserve the doc attribute’s escaped string representation, or emit valid comments for every line while preserving the documentation value.

### 2. Invalid datetime formats panic instead of returning errors

Locations: [src/function.rs:246](/work/crate/src/function.rs:246), [src/function.rs:250](/work/crate/src/function.rs:250), [src/config.rs:137](/work/crate/src/config.rs:137).

Expected: malformed format strings produce actionable function or configuration errors.

Observed: `datetime('%Q')`, `datetime_utc('%')`, and `--timestamp --timestamp-format %Q` each panic with “a Display implementation returned an error unexpectedly” and exit 101. The unchecked formatting error also escapes the public `run()` API.

Evidence: three executed regression cases reproduced these panics. Existing datetime tests cover successful formats only.

validate Chrono format items or use fallible formatting, returning a normal diagnostic.

### 3. Ambiguous module directories outside the project cause a panic

[src/compiler.rs:204](/work/crate/src/compiler.rs:204).

Expected: an ambiguous module source produces `AmbiguousModuleFile`, including useful candidate paths.

Observed: with `mod child '../shared'` and both `shared/justfile` and `shared/.justfile` present, `just --list` panics on `StripPrefixError` and exits 101. Valid external module paths need not start with the importing file’s parent directory.

Evidence: `test_ambiguous_external_module_directory_returns_diagnostic` reproduced the panic.

make candidate-path presentation fallible; retain absolute paths when stripping the parent prefix is impossible.

## Medium

### 4. Formatting removes `eager` and silently skips required evaluation

[src/assignment.rs:7](/work/crate/src/assignment.rs:7).

Expected: formatting preserves the existing guarantee that eager assignments run even with `set lazy`.

Observed: `eager x := …` is rewritten as `x := …`. An unused assignment that appended a marker before formatting stopped appending afterward, although the recipe still succeeded.

Evidence: `test_format_preserves_eager_assignment` expected two marker lines across two executions separated by `--fmt`; only one existed. The existing eager-evaluation test does not format its input.

serialize the `eager` modifier and add a regression comparing execution before and after formatting.

### 5. Inherited bindings override current-module assignments during lookup

[src/evaluator.rs:411](/work/crate/src/evaluator.rs:411).

Expected: references resolve to the current module’s assignments before inherited values or predefined constants.

Observed: when the parent exports `x := 'parent'`, and a child defines `a := x` and `x := 'child'`, running the child prints `parent child`. Similarly, `A := HEX` with `HEX := 'custom'` evaluates `A` to the predefined hexadecimal alphabet.

The evaluator checks recursively inherited scope values before evaluating the current module’s declaration. Results can therefore depend on names, evaluation order, and which command selected the variables.

Evidence: both scenarios failed executed regressions.

distinguish local evaluated bindings from inherited bindings, and resolve current-module declarations before falling back to inherited values.

### 6. Reusing a module source conflates distinct module identities

[src/compiler.rs:19](/work/crate/src/compiler.rs:19).

Expected: `mod a 'shared.just'` and `mod b 'shared.just'` create distinct module instances with their respective paths.

Observed: a shared recipe printing `module_path()` prints `b` when invoked through either module. Invoking both recipes prints only one line because execution deduplication treats them as the same recipe. An `a::x` override was also ineffective.

Evidence: `test_reused_module_sources_have_distinct_identity` expected `a\nb\n`; observed `b\n`.

separate source-text caching from module instantiation, preserving module path and binding identity for each declaration.

### 7. User-defined functions discard module bindings and invocation context

[src/evaluator.rs:226](/work/crate/src/evaluator.rs:226).

Expected: functions referencing module assignments use their established values and overrides; invocation-sensitive built-ins retain the current recipe context.

Observed:
- With `x=overridden`, a recipe prints `overridden original` for `x` and `f() := x`.
- Referencing one backtick-backed global directly and through two function calls executes its backtick three times.
- A function parameter named `x` changes the value of a separate global `y := x`.
- In a dependency recipe, direct `is_dependency()` returns `true`, while a wrapper function returns `false`.

Evidence: four executed regression tests reproduced these results. Functions are documented as unstable, but their documented module-assignment access still produces incorrect values.

use the defining module’s bindings and overrides, place parameters in a separate scope, and preserve invocation context.

### 8. Relative and unnormalized search ceilings are ignored

Locations: [src/config.rs:295](/work/crate/src/config.rs:295), [src/search.rs:196](/work/crate/src/search.rs:196).

Expected: equivalent paths identify the same ceiling, enforcing “Do not ascend above” that directory.

Observed: from a child directory, `--ceiling .` finds and executes a parent justfile. An absolute ceiling containing `child/../child` behaves likewise. The normalized absolute path correctly stops the search with “No justfile found.”

Evidence: executed regressions covered both ignored forms and the working absolute-path control.

resolve and normalize the ceiling against the invocation directory before comparing it with searched directories. Apply the same handling to project-root discovery.

### 9. Formatting removes module and alias privacy

Locations: [src/item.rs:59](/work/crate/src/item.rs:59), [src/alias.rs:41](/work/crate/src/alias.rs:41).

Expected: `--fmt` preserves `[private]`, including its effect on listing and completion.

Observed: formatting removes `[private]` from modules and aliases. A previously hidden module and its recipes subsequently appear in `--list --list-submodules`; a hidden alias appears in listing output.

Evidence: separate executed regressions reproduced both changes. The existing private-module test explicitly disables round-trip checking; the general round-trip helper checks textual idempotence rather than behavioral preservation.

serialize privacy attributes and test discovery output before and after formatting.

## Low

### 10. Strict Clippy fails on the supplied newer toolchain

Locations: [src/alias.rs:17](/work/crate/src/alias.rs:17), [src/invocation_parser.rs:333](/work/crate/src/invocation_parser.rs:333), also lines 404 and 648.

Observed: Clippy 0.1.99 rejects one `manual_assert_eq` occurrence and three `assert_is_empty` occurrences in `src/`; another occurs in supporting integration tests.

Consequence: the strict lint gate fails despite a successful build.

Evidence: strict Clippy failed. Repeating it with only these two lints allowed passed the entire workspace.

update the assertions. This is reported as toolchain compatibility debt, not a correctness failure or a demonstrated MSRV failure.

### 11. Private-item rustdoc fails on malformed Markdown

Locations: [src/lexer.rs:807](/work/crate/src/lexer.rs:807), [src/lexer.rs:824](/work/crate/src/lexer.rs:824), [src/lexer.rs:851](/work/crate/src/lexer.rs:851), line 852.

Expected: the strict documentation check can document private implementation items.

Observed: regex-like prose is interpreted as an unresolved intra-doc link and three undefined footnotes. With warnings denied, documentation generation fails.

Evidence: strict private-item rustdoc failed with four diagnostics. Strict public-only documentation passed.

render these patterns as code.

## Verification

Commands ran from `/work/crate` with `RUSTUP_AUTO_INSTALL=0`, `CARGO_NET_OFFLINE=true`, and `CARGO_BUILD_JOBS=2`. Installed versions were Rust/Cargo 1.99.1, Clippy 0.1.99, and rustfmt 1.10.0.

- `cargo build --locked --bin just` — passed.
- `cargo test --locked` — 513 unit tests passed; integration results were 1,240 passed, 5 failed, 8 ignored.
- `bash /work/[tooling]/scripts/[tooling] --audit /work/crate` — formatting passed; Clippy and private rustdoc failed as described; tests repeated the same five failures.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings -A clippy::manual_assert_eq -A clippy::assert_is_empty` — passed.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features` — passed.
- `python3 -I /work/[tooling]/scripts/[tooling] /work/crate` — completed; its 60 print-macro candidates reflect this application’s CLI output and were not counted as defects.
- `python3 /work/tmp/review_regressions.py` — 15 regression tests produced 17 expected assertion failures reproducing the behavioral findings. Inputs and diagnostics remain in `/work/tmp/review-regressions` and `/work/tmp/regressions.log`.

The five pre-existing baseline test failures were attributable to this environment:

- Missing `USER` and `XDG_RUNTIME_DIR` caused two failures.
- The sandbox permits writing a mode-0400 file, defeating `format::write_error`.
- Creating non-Unicode directory names returns OS error 84, blocking two tests before they invoke the crate.

After creating `/work/tmp/runtime`, this command passed 513 unit tests and 1,242 integration tests, with 8 ignored and the three filesystem-blocked tests filtered:
```sh
USER=audit XDG_RUNTIME_DIR=/work/tmp/runtime cargo test --locked -- \
  --skip format::write_error \
  --skip non_unicode::warn_for_non_unicode_invocation_directory \
  --skip non_unicode::warn_for_non_unicode_justfile_path
```

`cargo +nightly-2026-10-01 miri test --lib` stopped because isolation disallowed `getcwd`. Retrying with `MIRIFLAGS=-Zmiri-disable-isolation` ended with exit 137 before completion; that result is inconclusive and is not evidence of a defect.

Final count: 11 findings — 3 High, 6 Medium, 2 Low. Coverage was native aarch64 Linux; MSRV 1.85.0 and other-platform checks were unrun because those toolchains/targets were unavailable. Library doctests are disabled by the manifest. No dedicated security assessment, exploit work, fuzz campaign, or stress testing was performed. Last `/work/clock` reading: 11.8324 active minutes of 45.