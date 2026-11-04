# Audit report: just 1.50.0 (`crate/src/`)

## Summary

`just` holds up well on everyday input. The formatter round-trips (`--fmt` / `--dump`), all built-in functions are correct, and modules, imports, overrides and listing behave as documented. I found 5 Medium and 5 Low defects, plus 9 other notes items. There are no High:
- **Panics:** two crashes on unusual input. One is a lexer assertion; the other is `--show`/`--usage` given a whitespace-only path.
- **Wrong behaviour:** overrides are ignored inside user-defined functions; CRLF justfiles reject a documented escape; `&&` dependencies break the documented "runs once" rule.
- **Test suite:** the only failures are two tests that need environment variables this container doesn't set.

## High

None.

## Medium

**Overrides ignored inside user-defined functions**
`src/evaluator.rs:219-253` (`evaluate_defined_function`).
inside a function body, a global variable ignores `x=new` and `--set x new`, and backticks run again on every call.
each call builds a fresh `Evaluator` with an empty overrides map (line 241), a new root scope and `is_dependency: false` (line 246). Globals are therefore re-evaluated from source, not read from the module scope already evaluated.

- With `x := "orig"`, `f(a) := a + "-" + x`, `y := f("arg")`, the command `just --set x new2 --evaluate y` prints `arg-orig`.
- `just x=new r` where `r: (s f("d") g())` prints `s d-orig …`.
- With `x := \`echo tick >> /tmp/count; echo v\``, `f() := x`, `y := f()` and `z := f()`, the backtick ran 3 times instead of once.
- `g() := is_dependency()` returns `false` when used in dependency arguments.

**`--show " "` and `--usage " "` panic**
`src/subcommand.rs:843` (`resolve_path`), reached via `src/config.rs:89-104` (`parse_modulepath`).
the process panics with `attempt to subtract with overflow` (exit 101) instead of reporting an error.
`parse_modulepath` splits a whitespace-only argument into an empty path, which `Modulepath::try_from` accepts as the root. `resolve_path` then computes `path.components.len() - 1`.
`just --show " "` and `just --usage "  "` both panic at `src/subcommand.rs:843:37`.

**Lexer assertion panic on a trailing backslash at end of file**
`src/lexer.rs:763-785` (`lex_escape`); the panic fires at `src/lexer.rs:610` (`lex_dedent`).

- An indented non-recipe line ending in a bare `\` with no final newline crashes `just`.
- Without indentation, a final `\` is silently dropped (`a: b \` and `"a:\n  echo hi\n\\"` compile).
when `\` is the last character, `lex_escape` consumes it and returns `Ok(())` without emitting a token. The pending one-byte lexeme then trips `assert_eq!(self.current_token_length(), 0)` when dedenting at end of file, or is absorbed into the `Eof` token.
a justfile containing exactly a space then `\` makes `just --dump` panic with `assertion left == right failed, left: 1, right: 0` at `src/lexer.rs:610:5`. The fuzz harness found it.

**Escaped line break in a double-quoted string rejected in CRLF files**
`src/parser.rs:1016-1027` (`cook_string`).
the README (lines 1615-1634) documents `"\<newline>"` as an escaped line break. In a CRLF file it fails with "`\r` is not a valid escape sequence", although the same file with LF line endings compiles. The lexer treats `\` followed by CRLF as a continuation everywhere else (`lex_escape` lines 772-779).
the backslash state handles `'\n'` but not `'\r'` followed by `'\n'`.
a file containing `y := "q\` CRLF `r"` CRLF fails at `crlf:5:6`. With LF instead it evaluates to `qr`.

**Subsequent (`&&`) dependencies re-run recipes that already ran**
`src/justfile.rs:478-487` (`run_recipe`).
the README (around line 3153) says a recipe with the same arguments runs only once per invocation, "regardless of how many times it appears … as a dependency". A recipe reached through a subsequent dependency runs again.
subsequents run with `&Ran::default()` instead of the shared `ran` tracker.
with `a: && post`, `post: c`, `c:`, `b: c a`, running `just b` prints `c a c post b`, so `c` runs twice.

## Low

**`--choose` offers recipes from private modules**
`src/subcommand.rs:232` (`choose`).
recipes in a `[private] mod` reach the chooser, while `--list` and `--summary` hide them. The README says private recipes are skipped.
`stack.extend(module.modules.values())` instead of `module.public_modules(config)`.
with `[private] mod hidden`, the chooser input contained `hidden secret`.

**SIGTERM can be forwarded to an already-reaped PID**
`src/signal_handler.rs:93-106` and `135-139`.
`spawn` reaps the child in `f(child)` and only then re-locks to remove its PID. A SIGTERM handled in that window is sent to a PID the kernel may already have reused.
the PID is removed at line 139, after the child has been waited on at line 135.

**Fuzz target does not compile**
`src/lib.rs:174-186`, `src/fuzzing.rs:4`.
`fuzzing::compile` calls `testing::compile`, but `testing` is only compiled under `cfg(test)`.
`RUSTFLAGS="--cfg fuzzing" cargo build` fails with "cannot find module or crate `testing`".

**Mixed-whitespace check in recipe bodies is unreachable**
`src/lexer.rs:351-356`.
`body_whitespace` takes `.next()` of the char indices, which is always 0, so the slice is always empty. The two body-whitespace branches (lines 373 and 381) can never fire.
a recipe indented with two spaces, followed by a line indented with space-tab, reports "inconsistent leading whitespace" rather than the mixed-whitespace error.

**Positional override doc comment is stale**
`src/positional.rs:10`, `33` and `79-82`.
the docs say overrides match `[a-zA-Z_][a-zA-Z0-9_-]*=.*`, but any value containing `=` is now treated as an override. As a result, `just ./run=1` fails with "Invalid override path `./run`" instead of treating `./` as the search directory.

## Other notes

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

## Other notes

- I couldn't confirm whether the subsequent-dependency behaviour is intended. It could not be settled from the repository.

## Repository content addressed to agents
