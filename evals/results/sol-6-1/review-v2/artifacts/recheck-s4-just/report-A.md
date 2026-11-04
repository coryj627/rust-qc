Review found **6 issues: 4 Major and 2 Minor; no Blockers**. The binary builds, and the adjusted test suite passes, but local reproducers confirm incorrect module identity, function evaluation and error handling. No supplied source was edited.

## Major

### 1. Modules sharing a file lose their separate identities

Location: [src/compiler.rs:19](/work/crate/src/compiler.rs:19).

The compiler caches parsed files by filesystem path, although parsing also records the module path. With:

```just
mod a 'shared.just'
mod b 'shared.just'
```

both modules inherit the same internal identity.

Expected: invoking `a::show b::show` executes both recipes in their respective module contexts.

Actual: the reproducer prints only `b:default`. With separate overrides for `a::x` and `b::x`, it prints only `b:B`, instead of `a:A` followed by `b:B`.

Impact: ordinary invocations silently skip recipes and apply the wrong module configuration.

Fix: separate reusable source data from module-specific identity, or include module context in the compilation cache key.

### 2. User-defined functions discard module variable state

Location: [src/evaluator.rs:226](/work/crate/src/evaluator.rs:226), [src/evaluator.rs:241](/work/crate/src/evaluator.rs:241).

Function evaluation creates a fresh root scope and empty override map, then evaluates module assignments again.

Expected: functions referencing module assignments observe their established values, including command-line overrides.

Actual, confirmed by local tests:

- `x := 'original'; f() := x` still returns `original` with `x=override`.
- An assignment initialized with `uuid()` has a different value inside `f()`.
- With `y := 'global'; x := y; f(y) := x`, calling `f('parameter')` returns `parameter`, rather than `global`.
- A backtick inside a function cannot access an exported module variable that is available to the recipe.

Impact: extracting an expression into a function can change values, repeat commands or make working recipes fail.

Fix: preserve module bindings, overrides and memoization. Keep function parameters in a separate lexical scope so they cannot alter module assignment evaluation.

### 3. Functions incorrectly reset dependency status

Location: [src/evaluator.rs:246](/work/crate/src/evaluator.rs:246).

Every user-defined function evaluator sets `is_dependency` to `false`.

Expected: a wrapper around `is_dependency()` reports the current recipe’s dependency status, as documented in README.md:1940.

Actual: a dependency printing the built-in result and wrapper result produces `true:false`; the directly invoked recipe produces `false:false`.

Impact: dependency-specific behavior changes when placed inside a helper function.

Fix: propagate the caller’s dependency status into the function evaluator.

### 4. Invalid date/time formats panic

Locations: [src/function.rs:246](/work/crate/src/function.rs:246), [src/function.rs:250](/work/crate/src/function.rs:250), [src/config.rs:140](/work/crate/src/config.rs:140).

Expected: an invalid format produces a normal diagnostic and unsuccessful result.

Actual: all three ordinary inputs terminate with exit code 101 and `a Display implementation returned an error unexpectedly`:

- `datetime('%Q')`
- `datetime_utc('%Q')`
- `--timestamp --timestamp-format '%Q'`

Impact: a format typo crashes execution and bypasses normal error handling.

Fix: validate format items and use fallible formatting, returning the existing function/configuration error types.

## Minor

### 5. Clippy fails on four assertions

Locations: [src/alias.rs:17](/work/crate/src/alias.rs:17); src/invocation_parser.rs:333, :404 and :648.

Expected: the configured Clippy check succeeds.

Actual: Rust 1.99.1 rejects one equality assertion and three empty-collection assertions through `manual_assert_eq` and `assert_is_empty`. Allowing only those two lints makes the full workspace check pass.

Impact: the lint gate fails on the installed toolchain despite a successful build.

Fix: update the assertions to the suggested forms. Compatibility with older Clippy versions was not tested.

### 6. Four documentation comments break strict internal documentation

Locations: [src/lexer.rs:807](/work/crate/src/lexer.rs:807), :824, :851 and :852.

Expected: regex notation renders literally.

Actual: rustdoc interprets brackets as links or footnotes, producing four errors with warnings denied and private items documented.

Impact: strict internal documentation generation fails. Public documentation generation passes.

Fix: put the regex notation in code spans or fenced code blocks.

## Verification and reproduction

Commands ran from `/work/crate`, with `RUSTUP_AUTO_INSTALL=0` and `CARGO_NET_OFFLINE=true`.

- `cargo build --locked --bin just` — **passed**.
- `cargo test --locked` — **513 unit tests passed**; integration results: **1,240 passed, 5 failed, 8 ignored**.
- The five failures were traced to absent `USER`/`XDG_RUNTIME_DIR`, sandbox writes succeeding on read-only files, and rejection of non-Unicode directory creation before `just` ran.
- The adjusted run below **passed: 513 unit tests and 1,242 integration tests**, with three excluded and eight ignored:

```sh
USER=root XDG_RUNTIME_DIR=/work/tmp/review/runtime \
cargo test --locked -- \
  --skip format::write_error \
  --skip non_unicode::warn_for_non_unicode_invocation_directory \
  --skip non_unicode::warn_for_non_unicode_justfile_path
```

Additional checks:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps \
  --document-private-items --all-features
cargo test --workspace --all-features
```

Formatting passed. Clippy and internal rustdoc failed as described above. Workspace tests encountered the same five environment-dependent failures.

Public rustdoc passed. Clippy passed with only the two identified lints allowed:

```sh
cargo clippy --locked --workspace --all-targets --all-features -- \
  -D warnings -A clippy::manual_assert_eq -A clippy::assert_is_empty
```

All behavioral reproducers and their expected/actual outputs are retained locally:

```sh
python3 /work/tmp/review/reproduce.py
```

Miri passed four `unindent` tests. Its repeated-`run` test was blocked by Miri isolation prohibiting `mkdir`; this is not evidence of a crate defect.

The review combined source inspection with targeted CLI tests; it was not exhaustive. Other platforms and the declared Rust 1.85 MSRV were not executed. Dedicated security testing and inquiries requiring it remained outside scope; no exploitability investigation was performed.

Last `/work/clock` reading: **10.378 minutes used of 15**.