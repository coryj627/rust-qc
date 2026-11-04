Confirmed 4 Blocker, 3 Major, and 1 Minor findings. All behavior findings have reproductions. No crate files were edited; probes are under `/work/tmp`.

## Blocker

### Formatting silently removes semantic modifiers

Location: [assignment.rs:12](/work/crate/src/assignment.rs:12), [item.rs:59](/work/crate/src/item.rs:59).

`just --unstable --fmt` removes `eager` from assignments and `[private]` from modules.

For example:

```just
set lazy
eager x := `exit 1`
foo:
```

`just foo` fails before formatting and succeeds afterward because the check is skipped. Formatting a private module also makes its recipes appear in public listings.

Fix: preserve both flags when rendering, with round-trip tests that check behavior and visibility.

### Invalid date formats panic

Location: [function.rs:245](/work/crate/src/function.rs:245), [config.rs:135](/work/crate/src/config.rs:135).

Evaluating `x := datetime('%Q')` exits with a formatting panic instead of a function error. `datetime_utc()` and `--timestamp --timestamp-format '%Q'` have the same problem. The panic also escapes the public `run()` interface.

Fix: validate format strings or use fallible formatting and return a normal diagnostic.

### An indented trailing backslash panics during parsing

Location: [lexer.rs:763](/work/crate/src/lexer.rs:763), [lexer.rs:610](/work/crate/src/lexer.rs:610).

A file containing exactly two bytes—space followed by backslash, without a newline—makes `just --dump` panic. The public `summary()` function also panics.

`lex_escape()` consumes the backslash at EOF without emitting a token, violating the subsequent dedent assertion.

Fix: handle EOF explicitly, returning a syntax error or consuming the pending token consistently.

### Variable cycles through functions abort the process

Location: [assignment_resolver.rs:60](/work/crate/src/assignment_resolver.rs:60), [evaluator.rs:219](/work/crate/src/evaluator.rs:219).

This input passes compilation but aborts with stack overflow when evaluating `x`:

```just
set unstable
x := f()
f() := x
```

Call resolution checks function existence and arity without following dependencies through function bodies. Evaluation has no corresponding cycle guard.

Fix: track in-progress assignments across function calls and bound function-call depth, returning useful errors.

## Major

### Modules sharing a source file receive the same identity

Location: [compiler.rs:18](/work/crate/src/compiler.rs:18).

Given:

```just
mod a 'shared.just'
mod b 'shared.just'
```

and a shared recipe that prints `module_path()`, invoking `just a run b run` prints only `b`. Both modules receive path `b`, and execution deduplication suppresses the second invocation. Module-specific overrides are also affected.

Fix: distinguish source-file caching from module-instance identity and assign paths and binding identities per instance.

### Lazy dependency discovery takes exponential time

Location: [justfile.rs:187](/work/crate/src/justfile.rs:187).

The traversal revisits shared dependencies for every path through the graph. A 599-byte file containing 24 layers of shared dependencies took about 8 seconds with `set lazy`; without it, execution took about 3 milliseconds.

Fix: maintain a visited set while collecting recipe variable references.

### Functions discard evaluated module state

Location: [evaluator.rs:226](/work/crate/src/evaluator.rs:226).

For:

```just
set unstable
x := 'original'
f() := x
a := f()
```

`just --set x override --evaluate a` prints `original`.

Each function creates a fresh root scope and empty overrides, causing module assignments to be reevaluated. Exported module variables are likewise unavailable to shell calls inside functions.

Fix: evaluate function parameters over the appropriate module scope, retaining overrides and existing bindings.

## Minor

### Verification fails on the supplied toolchain

Location: [alias.rs:17](/work/crate/src/alias.rs:17), [invocation_parser.rs:333](/work/crate/src/invocation_parser.rs:333), [lexer.rs:807](/work/crate/src/lexer.rs:807).

Rust 1.99.1 reports four denied Clippy diagnostics involving assertions. Strict rustdoc reports four errors caused by regex notation interpreted as links or footnotes.

Fix: update assertion forms and put regex documentation in code spans.

## Verification

- `cargo build --locked --bin just`: passed.
- `cargo fmt --all --check`: passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: failed as described above.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --all-features`: failed as described above.
- `cargo test --workspace --all-features`: 513 unit tests passed; integration results were 1,240 passed, five failed, eight ignored. Two failures disappeared after setting `USER` and `XDG_RUNTIME_DIR`. Excluding three tests requiring unavailable permission/non-Unicode filesystem behavior, the adjusted run passed all 1,242 integration tests.
- `cargo +nightly-2026-10-01 miri test --locked --lib`: stopped at Miri’s isolated `getcwd` restriction; no soundness conclusion.
- A completed 100,000-input parser probe found 12 instances of the trailing-backslash panic. A larger run exceeded its resource limit and was not treated as defect evidence.

Windows and Rust 1.85 were not checked because those targets/toolchains are unavailable.