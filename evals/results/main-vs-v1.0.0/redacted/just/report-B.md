## High

**`--fmt` silently deletes comments**
`src/parser.rs:228-229` (`expect_eol`), called from `src/parser.rs:1230-1231` and the attribute parser.
`expect_eol` reads the trailing comment token and throws it away, so the formatter drops comments that follow a recipe header or an attribute. `--fmt` is a stable command (it needs no `--unstable`) and it rewrites the user's file. Input:
```
foo: # after colon
bar a="1": foo # after dep
[private] # c1
[group("g")] # attr comment
```
After `just --fmt`, all four comments are gone. Comments after `set`, `alias`, `export`, `mod`, `import` and assignments are kept.

## Medium

**Lexer panics on a backslash at end of file after an indented line**
`src/lexer.rs:763-785` (`lex_escape`); the failing assertion is at `src/lexer.rs:610`.
if `\` is the last byte of the file, `lex_escape` returns without emitting a token. The dedent pass at EOF then fails `assert_eq!(self.current_token_length(), 0)`. The 5-byte justfile `a\n  \` makes `just` panic. A file containing only `\` is accepted as "no recipes". My mutation harness (30 million inputs through `Compiler::test_compile`) found this. It was the only panic it found.

**Stack overflow on long operator chains and long dependency chains**
`src/parser.rs:654-720`, `src/recipe_resolver.rs:39-112`.
the parser's depth limit (256) only counts `parse_expression`. The recursion through `+`, `/`, `&&` and `else if` (`parse_conjunct`, `parse_disjunct`, `parse_conditional`) is never counted. Results:
- In a release build, `x := "a" + "a" + …` with 20,000 terms aborts with "has overflowed its stack" instead of giving the "Parsing recursion depth exceeded" error.
- A debug build aborts at 1,000 terms, and on 1,000 chained `else if`s.
- A chain of 5,000 recipes, each depending on the next, aborts in a release build, even for `--summary`.

**`datetime()`, `datetime_utc()` and `--timestamp-format` panic on an invalid format**
`src/function.rs:245-251`, `src/config.rs:136-142`.
chrono's formatter returns `fmt::Error` for an unknown specifier, and `.to_string()` then panics. `x := datetime("%Q")` and `just --timestamp --timestamp-format %Q` both panic with "a Display implementation returned an error unexpectedly" (exit 101).

**Listing commands panic when stdout is closed early**
`print!`/`println!` in `src/subcommand.rs` (for example lines 129, 131, 163, 206, 309) and `src/justfile.rs:307-328`.
on a large justfile, `just --list | head -c 10` panics with "failed printing to stdout: Broken pipe". So do `--summary`, `--dump`, `--evaluate` and `--variables`. `--changelog` and `--dump-format json` already handle this correctly.

**`--choose` breaks on paths with quotes, and allows shell injection**
`src/subcommand.rs:239-247`.
the default chooser command splices the justfile path into `fzf --preview 'just … --justfile "PATH" --show {}'` without escaping it. In a directory named `it's dir` this fails with "Syntax error: Unterminated quoted string". A directory name containing `'$(cmd)'` would run `cmd`.

**A missing `dotenv-path` falls back to an ancestor `.env`, even with `dotenv-required`**
`src/load_dotenv.rs:27-41`; the documentation is at `README.md:1039`.
if the `dotenv-path` file does not exist, the code goes on to search ancestor directories for `.env`. With `set dotenv-path := "missing.env"` plus `set dotenv-required`, `just` loaded `FOO=parent` from `../.env` and exited 0. The settings table says `dotenv-path` should "error if not present".

**Format strings unindent after interpolation and escape processing**
`src/evaluator.rs:375-388` (the `FormatString` arm), compared with `src/parser.rs:939-943`.
indented format strings call `unindent()` on the final value, so interpolated values and escape output change the indentation that gets removed. With `x := "line1\nline2"`, `f"""` / `    a {{x}}` / `    b` / `"""` evaluates to `"    a line1\nline2\n    b\n"`, with no unindent at all. The README says escape sequences are processed after unindentation, so escape output does not count; indented strings without interpolation already behave that way.

**A relative `--ceiling` or `JUST_CEILING` is silently ignored**
`src/search.rs:191-195` and `src/search.rs:245-249`; the value is passed through unchanged at `src/config.rs:295`.
the code compares `directory == ceiling` against absolute ancestor paths without resolving the ceiling first. A relative ceiling therefore never matches. In a `sub/` directory with no justfile, `just --ceiling .` ran the parent directory's justfile; `--ceiling "$PWD"` correctly stopped. A ceiling given through a symlinked path probably has the same problem.

**`--edit` fails when `$VISUAL` or `$EDITOR` includes arguments**
`src/subcommand.rs:329-337`.
the whole variable is used as the program name. With `VISUAL="true --wait"`, `just --edit` fails with "Editor `true --wait` invocation failed: No such file or directory". Values like `code --wait` are common.

**The crate's fuzz target does not build and could not work as written**
`src/fuzzing.rs:1-5`, `src/lib.rs:174-186`, `src/testing.rs:3-5`.
`fuzzing` is compiled under `cfg(fuzzing)` but calls `testing::compile`, and `testing` only exists under `cfg(test)`. `RUSTFLAGS="--cfg fuzzing" cargo check --lib` fails with E0433. Even if it built, `testing::compile` calls `expect("expected successful compilation")`, so every invalid input would be reported as a crash.

## Low

**An empty shebang line is reported as an internal error**
`src/recipe.rs:485-486`, `src/shebang.rs:23`.
a body starting with just `#!` is classified as a shebang recipe but rejected at run time with "Internal runtime error, this may indicate a bug in just: bad shebang line".

**`which()` rejects a trailing `;` in `PATHEXT` (Windows, found by reading)**
`src/which.rs:40-45`.
an empty entry, such as a trailing `;`, fails with "`PATHEXT` entry `` does not start with `.`". `Command::resolve` (`src/command_ext.rs:104-110`) skips such entries.

**`--fmt` rewrites CRLF files to LF, and `--fmt --check` shows a diff you can't see**
`src/subcommand.rs:363-397`.
an already-formatted CRLF justfile fails `--check` with lines that look identical, and `--fmt` converts every line ending.

**Mixed-whitespace check in recipe bodies never runs**
`src/lexer.rs:351-356`.
`body_whitespace` is sliced as `whitespace[..i]`, where `i` comes from `.take(n).map(|(i, _)| i).next()`. That is always 0, so `body_whitespace` is always empty and the body `Mixed` branch can never fire. Mixed lines are reported as "inconsistent" instead.

**Error messages name the wrong path**
`src/function.rs:142-145` (`absolute_path`), `src/function.rs:583-588` and `603-605` (`source_directory`, `source_file`).
the messages print `search.working_directory` or the raw token path, not the path that actually failed.

**Toolchain drift and rustdoc errors**
`src/alias.rs:17`, `src/invocation_parser.rs:333,404,648`; `src/lexer.rs:807,824,851,852`.
clippy 1.99 adds `manual_assert_eq` and `assert_is_empty` under pedantic, and they fail the clippy step. Rustdoc with private items rejects the regex text in the lexer doc comments as a broken link and broken footnotes. CI doesn't run rustdoc.

**Tests depend on the host environment**
`tests/functions.rs:88`, `tests/directories.rs:97`.
these tests unwrap `USER` and expect `XDG_RUNTIME_DIR` to exist, so they fail in a clean environment. Those two were the only failures; 1,243 integration tests and 513 unit tests passed.

**Lint hygiene**
across `src/`.
the advisory lint pass flagged:
- about 90 `unwrap()` calls, for example `src/function.rs:448`, `577`, `580` and `src/subcommand.rs:146`, `166-171`;
- 11 assertions without a message;
- missing crate docs.

The 102 wildcard imports are this project's convention (`use super::*`), so I didn't count them as a finding.

## Low

- `choose("4000000000", "ab")` runs until the timeout and keeps using memory. A soft cap with an error would be friendlier.
- `--fmt` writes the file in place with `fs::write`. Writing a temporary file and renaming it would avoid a truncated justfile if the write fails.
- By reading: the signal thread in `src/platform/unix.rs:56-60` would loop forever, printing warnings, if reading the pipe failed repeatedly.
- The three `unsafe` blocks, all in `src/signals.rs`, have SAFETY comments that hold. The handler only calls async-signal-safe functions (`write`, `abort`).
