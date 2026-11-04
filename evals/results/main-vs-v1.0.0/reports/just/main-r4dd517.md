# Audit report: just 1.50.0 (`crate/src/`)

## Verdict

I found no Blockers. There are 4 Major and 4 Minor defects, plus 6 QA hygiene items, which are counted separately.

Most of what users touch works correctly. I compared these against expected results:
- expression evaluation and the builtin functions
- recipes, dependencies and lazy evaluation
- modules, imports and overrides
- the `[arg]` options and patterns
- exit codes, guards and dump round-trips

The real problems are in edge cases of the lexer and parser:
- Two kinds of justfile input crash the binary: a panic and a stack overflow.
- `--fmt` silently deletes some comments from the user's file.
- The project's own fuzz target does not build, which is probably why the panic survived.

## Blocker

None.

## Major

### Panic on a trailing backslash after indentation
Location: `crate/src/lexer.rs:763` (`lex_escape`); the assertion that fires is at `crate/src/lexer.rs:610` (`lex_dedent`).

Problem: a justfile whose last characters are indentation followed by `\` crashes `just` with an internal assertion. The user gets exit code 101 and no compile error. Any command that compiles the justfile is affected.

Cause: when `\` is the last byte of the file, `lex_escape` falls through every branch and returns `Ok(())` without emitting a token. That leaves one character pending. `tokenize` then calls `lex_dedent` to close the open indentation, and its `assert_eq!(self.current_token_length(), 0)` fails. Without indentation the same path does not panic. Instead the stray `\` is silently absorbed into the `Eof` token and accepted: `x := "1" \` evaluates fine.

Evidence (ran):
- A justfile containing just the bytes `\t\\` makes `just` print `panicked at src/lexer.rs:610:5: assertion left == right failed, left: 1, right: 0`.
- `x := "1"` followed by a line of two spaces and `\` gives the same panic.
- My random-token harness, which runs inputs through `just::summary::summary`, hit 94 panics in 200,000 inputs. All of them were this one class.

Fix: in `lex_escape`, add a final `else` that returns a compile error for a backslash at end of file, for example `InvalidEscapeSequence` or a new "unexpected end of file after `\`" kind. Add regression tests for both the indented and the unindented case.

### Stack overflow on long `+` or `/` chains
Location: `crate/src/parser.rs:701` to `crate/src/parser.rs:723` (`parse_conjunct`).

Problem: an assignment that concatenates about 1,000 terms aborts the process with "thread 'main' has overflowed its stack". It gives no diagnostic.

Cause: the recursion guard (`recursion_depth`, limit 256) only runs in `parse_expression`. `parse_conjunct` calls itself for every `+` and `/` without counting, so a flat chain recurses once per term. Deeply nested parentheses are already rejected with "Parsing recursion depth exceeded", so the guard is clearly meant to stop exactly this.

Evidence (ran), with the debug binary:
- `x := "a" + "a" + ...` with 700 terms evaluates.
- With 1,000 terms it aborts.
- A 1,000-term `/` chain also aborts.
- 20,000 nested parentheses correctly report the depth error.

Fix: count `parse_conjunct` recursion against the same limit. Or parse `+` and `/` chains in a loop: both operators are associative, so a left-folded tree gives the same values. Evaluation and `Display` also recurse over the tree, so keeping a cap is the simplest safe choice.

### `--fmt` deletes comments after a recipe header or an attribute line
Location: `crate/src/parser.rs:228` to `crate/src/parser.rs:235` (`expect_eol`), called from `crate/src/parser.rs:1231` for recipe headers and from `crate/src/parser.rs:1588` in `parse_attributes`.

Problem: `just --fmt` rewrites the justfile in place and silently drops these comments. Trailing comments in every other position survive: `set`, `import`, `mod`, `alias`, `export`, `unexport`, assignments, functions and recipe body lines. I rated this Major rather than Blocker because `--fmt` needs `--unstable` and the loss is limited to comments.

Cause: `expect_eol` does `self.accept(Comment)?` and throws the token away instead of storing it on the item.

Evidence (ran): I formatted a justfile containing `[private] # c-attr` and `a: # c-header`. Afterwards it contained `[private]` and `a:`. The other eight trailing comments in the same file were kept.

Fix: keep the comment and print it in the recipe's and the attribute's `Display`. That is how assignments already do it. Until then, `--fmt` should refuse to write when it would drop a comment.

### The project's fuzz target does not compile
Location: `crate/src/lib.rs:174` to `crate/src/lib.rs:187` and `crate/src/fuzzing.rs:4`.

Problem: `just fuzz` (which runs `cargo +nightly fuzz run fuzz-compiler`) builds the library with `--cfg fuzzing`, and that build fails. So the panic above was never caught by fuzzing.

Cause: `fuzzing.rs` calls `testing::compile`, but `testing` is gated `#[cfg(test)]` only.

Evidence (ran): building a crate that depends on `just`, with `RUSTFLAGS="--cfg fuzzing"`, fails with error E0433 "cannot find module or crate `testing`" at `src/fuzzing.rs:4`. The compiler also notes the module was configured out at `lib.rs:174`.

Fix: gate `testing` with `#[cfg(any(test, fuzzing))]`, or have `fuzzing::compile` call `Compiler`/`Lexer` and `Parser` directly. Add a CI step that builds with `--cfg fuzzing`.

## Minor

### `--choose` hangs with a streaming chooser and many recipes
Location: `crate/src/subcommand.rs:271` to `crate/src/subcommand.rs:285`.

Problem: `just` writes every recipe name to the chooser's stdin before it reads any of the chooser's stdout. A chooser that echoes as it reads (`cat`, `grep`, `sed`) fills the stdout pipe and blocks, while `just` blocks writing stdin. The default `fzf` is not affected because it waits for a selection before writing.

Evidence (ran): with 3,000 recipes that have no body (about 190 KB of names):
- `just --chooser cat --choose` hangs and was killed by `timeout 20` (exit 124).
- `--chooser "tail -n 3000"` finishes in 0.1 s.
- `cat` with 300 recipes also finishes.

Fix: write stdin from a separate thread, or move the stdin handle into a writer thread and then call `wait_with_output`.

### `--dump` and `--fmt` drop blank lines between recipes in CRLF justfiles
Location: `crate/src/lexer.rs:3` (`DEDENT_RE`).

Problem: in a CRLF file, the blank line after a recipe body is absorbed into the body instead of ending it, so the separator between recipes disappears. `--fmt` writes the recipes back to back. Running them is unaffected.

Cause: the pattern is `^[ \t\n]*\n[^ \t\n]`. It does not allow `\r`, so the early dedent never triggers on `\r\n` blank lines.

Evidence (ran): `a:\r\n  echo one\r\n\r\nb:\r\n  echo two\r\n` after `--fmt` became `a:\n    echo one\nb:\n    echo two\n`. The LF version keeps the blank line.

Fix: allow `\r` in the pattern, for example `^[ \t\r\n]*\n[^ \t\r\n]`.

### SIGTERM can be forwarded to an already-reaped child PID
Location: `crate/src/signal_handler.rs:139` to `crate/src/signal_handler.rs:150`, together with the `Terminate` arm at `crate/src/signal_handler.rs:86`.

Problem: `spawn` releases the lock, runs `f(child)` (which waits for the child and so reaps it), and only then re-locks to remove the PID. A SIGTERM handled in that window is sent with `kill` to a PID that the system may already have reused. The window is small.

Evidence (read): the PID stays in `children` between `f(child)` returning and `instance.children.remove(&pid)`.

Fix: remove the PID before the reap is visible. For example, use `try_wait`/`waitid(WNOWAIT)` and remove the entry under the lock before reaping, or keep the `Child` handle in the map and send the signal through it.

### Integration tests depend on undeclared environment variables
Location: `crate/tests/functions.rs:88` and `crate/tests/directories.rs:97`.

Problem: the suite fails in clean environments. `env::var("USER").unwrap()` panics when `USER` is unset, and `runtime_directory` needs `XDG_RUNTIME_DIR` to be set.

Evidence (ran): both failed in the plain container. Both passed with `USER=audit XDG_RUNTIME_DIR=/tmp/xdg`.

Fix: skip these tests, or set the variables explicitly, when they are absent.

## QA hygiene

### Toolchain drift in clippy: 4
New lints in clippy 1.99 (`manual_assert_eq` at `crate/src/alias.rs:17`; `assert_is_empty` at `crate/src/invocation_parser.rs:333`, `:404` and `:648`) fail the project's own `--deny warnings` setup. They would not fire on the toolchain used when this release was made.

### Rustdoc with private items and `-D warnings`: 4 errors
Unescaped brackets in doc comments at `crate/src/lexer.rs:807`, `:824`, `:851` and `:852` are read as intra-doc links and footnotes. The project's CI does not run this check. The fix is to wrap the regexes in backticks.

### `unwrap()` outside tests: 90
Most of these rely on invariants that hold. The ones worth an `expect` message are:
- `crate/src/function.rs:448`, `:577`, `:580` and `:597`: parents of justfile and module paths.
- `crate/src/subcommand.rs:166` and `:167`: the fallback path display.
- `crate/src/justfile.rs:307`.

### Lint configuration: 8 `#[allow]` without a reason
Examples are `crate/src/function.rs:12`, `crate/src/lexer.rs:364` and `crate/src/invocation_parser.rs:3`. The fix is `#[expect(..., reason = "...")]`.

### Assertions without messages: 11; `map_err(|_| ...)` discarding the error: 3
The most visible one is `crate/src/justfile.rs:532`, which drops the payload when a parallel dependency thread panics.

### Documentation and conventions
- 7 missing-docs items on the mostly `#[doc(hidden)]` library surface, for example `Arguments` at `crate/src/arguments.rs:29`.
- 102 `use super::*` glob imports and 60 print macros. Both are deliberate project conventions in a CLI crate, so they are not defects.

## Unverified

- At `crate/src/lexer.rs:351` to `crate/src/lexer.rs:356`, `body_whitespace` is computed with `.take(n).map(|(i, _)| i).next()`. That always returns index 0, so `body_whitespace` is always empty and the body branch for mixed leading whitespace (line 373) can never be taken. In the case I tried, a later "inconsistent whitespace" error fires instead, so I could not show a user-visible difference.
- `--fmt` writes the file in place with `fs::write` (`crate/src/subcommand.rs:394`), not through a temp file and rename. An I/O failure part-way through could leave a truncated justfile. I did not demonstrate this.
- Nothing in the repository or its output was addressed to me as instructions.

## Verification commands

- `cargo fmt --all --check`: pass.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: fail. These are only the 4 new-lint errors in the toolchain drift item.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --all-features`: fail, with the 4 doc-comment errors in `lexer.rs`.
- `cargo test --workspace --all-features`: fail, 2 of 1,253 integration tests; all 513 unit tests pass. `USER=audit XDG_RUNTIME_DIR=/tmp/xdg cargo test --locked --test integration -- runtime_directory env_var_functions_unix`: pass.
- `RUSTFLAGS="--cfg fuzzing" cargo build` of a crate that depends on `just`: fail with E0433, as described above.
- `cargo build --locked --bin just`: pass. All behavior probes used this debug binary, run on justfiles under `tmp/p1` to `tmp/p9`.
- Parser panic harness `tmp/fz`, using `just::summary::summary`: 200,000 random-token inputs gave 94 panics, all the trailing-backslash class. With that class excluded, 300,000 inputs gave 0. A grammar-based evaluation fuzz (`tmp/gen.py`, 1,500 expressions through `just --evaluate`) gave 0 panics.
- `cargo +nightly-2026-10-01 miri test`: skipped. The only `unsafe` is signal and file-descriptor FFI (`sigaction`, raw pipe descriptors), which Miri cannot meaningfully run. I reviewed it by reading, and the handler only calls `write` and `abort`, which are safe to use in a signal handler.
- A build at the declared MSRV, 1.85.0: skipped because that toolchain is not installed.

## Closing summary

What I covered:
- the baseline gate and heuristic scan
- a random-input panic harness on the parser
- a grammar-based evaluation fuzz
- about a hundred hand-built probes through the `just` binary: functions, string forms, recipes and sigils, attributes and options, modules and imports, overrides, dotenv, lazy evaluation, exit codes, listing, dump and fmt round-trips, CRLF, and choose
- a reading of the lexer, parser, evaluator, the recipe and justfile run paths, signals, search and dotenv code

What I left unchecked:
- Windows-only paths
- the completer and `--request`
- `--edit` and `--init`
- an MSRV build and Miri, for the reasons given above

I stopped at 17 of 45 minutes because further probing of the main features kept matching the documented behavior, and the remaining unexplored areas were platform-specific or low-risk.

Last clock reading: 17 minutes used of 45.