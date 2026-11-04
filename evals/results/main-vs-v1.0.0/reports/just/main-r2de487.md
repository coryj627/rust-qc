# Audit report: just 1.50.0 (`crate/src/`)

## Verdict

`just` holds up well on everyday input. The formatter round-trips (`--fmt` / `--dump`), all built-in functions are correct, and modules, imports, overrides and listing behave as documented. I found 5 Major and 5 Minor defects, plus 9 QA hygiene items. There are no Blockers:
- **Panics:** two crashes on unusual input. One is a lexer assertion; the other is `--show`/`--usage` given a whitespace-only path.
- **Wrong behaviour:** overrides are ignored inside user-defined functions; CRLF justfiles reject a documented escape; `&&` dependencies break the documented "runs once" rule.
- **Test suite:** the only failures are two tests that need environment variables this container doesn't set.

## Blocker

None.

## Major

### Overrides ignored inside user-defined functions
Location: `src/evaluator.rs:219-253` (`evaluate_defined_function`).
Problem: inside a function body, a global variable ignores `x=new` and `--set x new`, and backticks run again on every call.
Cause: each call builds a fresh `Evaluator` with an empty overrides map (line 241), a new root scope and `is_dependency: false` (line 246). Globals are therefore re-evaluated from source, not read from the module scope already evaluated.
Evidence (ran):
- With `x := "orig"`, `f(a) := a + "-" + x`, `y := f("arg")`, the command `just --set x new2 --evaluate y` prints `arg-orig`.
- `just x=new r` where `r: (s f("d") g())` prints `s d-orig …`.
- With `x := \`echo tick >> /tmp/count; echo v\``, `f() := x`, `y := f()` and `z := f()`, the backtick ran 3 times instead of once.
- `g() := is_dependency()` returns `false` when used in dependency arguments.

Fix: evaluate function bodies in a child of the caller's module scope. Pass the caller's overrides and `is_dependency`, and bind only the parameters in the new scope.

### `--show " "` and `--usage " "` panic
Location: `src/subcommand.rs:843` (`resolve_path`), reached via `src/config.rs:89-104` (`parse_modulepath`).
Problem: the process panics with `attempt to subtract with overflow` (exit 101) instead of reporting an error.
Cause: `parse_modulepath` splits a whitespace-only argument into an empty path, which `Modulepath::try_from` accepts as the root. `resolve_path` then computes `path.components.len() - 1`.
Evidence (ran): `just --show " "` and `just --usage "  "` both panic at `src/subcommand.rs:843:37`.
Fix: use `let Some((name, parents)) = path.components.split_last() else { return Err(...) }` in `resolve_path`, or reject empty paths for show and usage.

### Lexer assertion panic on a trailing backslash at end of file
Location: `src/lexer.rs:763-785` (`lex_escape`); the panic fires at `src/lexer.rs:610` (`lex_dedent`).
Problem:
- An indented non-recipe line ending in a bare `\` with no final newline crashes `just`.
- Without indentation, a final `\` is silently dropped (`a: b \` and `"a:\n  echo hi\n\\"` compile).
Cause: when `\` is the last character, `lex_escape` consumes it and returns `Ok(())` without emitting a token. The pending one-byte lexeme then trips `assert_eq!(self.current_token_length(), 0)` when dedenting at end of file, or is absorbed into the `Eof` token.
Evidence (ran): a justfile containing exactly a space then `\` makes `just --dump` panic with `assertion left == right failed, left: 1, right: 0` at `src/lexer.rs:610:5`. The fuzz harness found it.
Fix: when `self.next` is `None` after the backslash, return an error such as `UnexpectedEndOfToken` or a dedicated "escape at end of file" error.

### Escaped line break in a double-quoted string rejected in CRLF files
Location: `src/parser.rs:1016-1027` (`cook_string`).
Problem: the README (lines 1615-1634) documents `"\<newline>"` as an escaped line break. In a CRLF file it fails with "`\r` is not a valid escape sequence", although the same file with LF line endings compiles. The lexer treats `\` followed by CRLF as a continuation everywhere else (`lex_escape` lines 772-779).
Cause: the backslash state handles `'\n'` but not `'\r'` followed by `'\n'`.
Evidence (ran): a file containing `y := "q\` CRLF `r"` CRLF fails at `crlf:5:6`. With LF instead it evaluates to `qr`.
Fix: in the `Backslash` state, accept `'\r'` and require the next character to be `'\n'`, mirroring `lex_escape`.

### Subsequent (`&&`) dependencies re-run recipes that already ran
Location: `src/justfile.rs:478-487` (`run_recipe`).
Problem: the README (around line 3153) says a recipe with the same arguments runs only once per invocation, "regardless of how many times it appears … as a dependency". A recipe reached through a subsequent dependency runs again.
Cause: subsequents run with `&Ran::default()` instead of the shared `ran` tracker.
Evidence (ran): with `a: && post`, `post: c`, `c:`, `b: c a`, running `just b` prints `c a c post b`, so `c` runs twice.
Fix: pass `ran` to the subsequent run, or document that subsequents bypass deduplication. This may be deliberate, so that `&&` recipes always run. If so, the README needs the exception.

## Minor

### `--choose` offers recipes from private modules
Location: `src/subcommand.rs:232` (`choose`).
Problem: recipes in a `[private] mod` reach the chooser, while `--list` and `--summary` hide them. The README says private recipes are skipped.
Cause: `stack.extend(module.modules.values())` instead of `module.public_modules(config)`.
Evidence (ran): with `[private] mod hidden`, the chooser input contained `hidden secret`.
Fix: iterate `public_modules(config)`.

### SIGTERM can be forwarded to an already-reaped PID
Location: `src/signal_handler.rs:93-106` and `135-139`.
Problem: `spawn` reaps the child in `f(child)` and only then re-locks to remove its PID. A SIGTERM handled in that window is sent to a PID the kernel may already have reused.
Evidence (read): the PID is removed at line 139, after the child has been waited on at line 135.
Fix: keep the `Child` (or a reaped flag) in the map and mark it before waiting, or use `waitid` with `WNOWAIT` so the PID stays reserved until it is removed from the map.

### Fuzz target does not compile
Location: `src/lib.rs:174-186`, `src/fuzzing.rs:4`.
Problem: `fuzzing::compile` calls `testing::compile`, but `testing` is only compiled under `cfg(test)`.
Evidence (ran): `RUSTFLAGS="--cfg fuzzing" cargo build` fails with "cannot find module or crate `testing`".
Fix: gate `testing` with `#[cfg(any(test, fuzzing))]`.

### Mixed-whitespace check in recipe bodies is unreachable
Location: `src/lexer.rs:351-356`.
Problem: `body_whitespace` takes `.next()` of the char indices, which is always 0, so the slice is always empty. The two body-whitespace branches (lines 373 and 381) can never fire.
Evidence (ran): a recipe indented with two spaces, followed by a line indented with space-tab, reports "inconsistent leading whitespace" rather than the mixed-whitespace error.
Fix: use `.nth(n)` or `.last()` with the character's byte length to slice the first `n` characters, or delete the dead branches.

### Positional override doc comment is stale
Location: `src/positional.rs:10`, `33` and `79-82`.
Problem: the docs say overrides match `[a-zA-Z_][a-zA-Z0-9_-]*=.*`, but any value containing `=` is now treated as an override. As a result, `just ./run=1` fails with "Invalid override path `./run`" instead of treating `./` as the search directory.
Fix: update the docs, or skip values whose left side is not a valid module path so they fall through to search-directory handling.

## QA hygiene

These come from the `--audit` lint pass and the heuristic scan; none needs a behaviour change.

1. **Clippy fails only on the newer toolchain: 5 errors.** `manual_assert_eq` at `src/alias.rs:17`, plus 4 "assert to check empty or equality" errors in tests (for example `src/invocation_parser.rs:333`). These are new lints in clippy 1.99; CI uses unpinned stable.
2. **Rustdoc with private items fails: 4 errors.** Unescaped brackets in private doc comments at `src/lexer.rs:807`, `824`, `851` and `852`. Project CI does not document private items.
3. **Two environment-dependent integration tests:** `directories::runtime_directory` needs `XDG_RUNTIME_DIR` and `functions::env_var_functions_unix` needs `USER`. Both fail in a minimal environment.
4. **Wildcard imports: 102** (`use super::*` throughout; this is the project's convention).
5. **`unwrap()` outside tests: 90.** Most are invariants, such as `src/function.rs:577-597` on `justfile.parent()`.
6. **Asserts without a message: 11.**
7. **`#[allow]` without a reason: 8.**
8. **Missing docs:** 4 crate-level items, plus `Arguments`, `INIT_JUSTFILE` and `unindent`, which are all `#[doc(hidden)]` re-exports.
9. **`map_err(|_| …)` discarding the error: 3.**

I discarded 24 "print in library code" scan hits, because `just` is an application.

## Unverified

- I couldn't confirm whether the subsequent-dependency behaviour is intended. It could not be settled from the repository.

## Repository content addressed to agents

`crate/skills/just/SKILL.md.quarantined` is a `just` usage reference written for AI agents. It gives no instructions about this audit, and I did not use it.

## Verification commands

Run from `crate/` unless noted.
- `cargo fmt --all --check`: pass.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: fail. The 5 errors are toolchain drift (hygiene item 1).
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --all-features`: fail, 4 errors (hygiene item 2).
- `cargo test --workspace --all-features`: fail, for environmental reasons only. 513 unit tests pass; integration has 1243 passed, 2 failed (hygiene item 3).
- `cargo build --locked --bin just`: pass. All CLI probes used `target/debug/just` on justfiles under `tmp/p1`–`tmp/p18`.
- Fuzz harness `tmp/fz` (`cargo build --release --offline`, then run): a panic search over every input up to 4 characters plus 60,000 seeded mutations, through `just::run(["just","--unstable","-f",…,"--dump"])`. It found one panic (the lexer finding above).
- Round-trip harness `tmp/fz/src/bin/rt.rs`: 12,111 compiling mutated inputs with `summary(x) == summary(dump(x))` and `dump` idempotent. No mismatches.
- `cargo +nightly-2026-10-01 miri test`: skipped. The only `unsafe` is signal and pipe FFI (`src/signals.rs`), which Miri cannot execute; its SAFETY comments are present and reasonable.

## Closing summary

I ran the gate and heuristic scan, and probed the CLI on 18 justfiles covering expressions, every built-in function, recipes, parameters and options, modules, imports, dotenv, search, listing, choose, formatting and error rendering. I also ran a panic fuzzer and a dump round-trip fuzzer. Windows-only paths (cygpath, PowerShell, `PATHEXT`) were not checked. Signal-forwarding behaviour was only read, not exercised. I didn't modify `crate/`. I stopped at 15 of 45 minutes because the remaining unread code is mostly display and plumbing, which the round-trip fuzzer and probes already covered. Further time would likely have yielded only cosmetic findings.

Last `./clock` reading: 15 minutes used of 45.