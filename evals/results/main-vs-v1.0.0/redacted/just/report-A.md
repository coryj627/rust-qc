## Medium

**Trailing backslash at end of file panics the lexer or is silently dropped**
`src/lexer.rs:763` (`lex_escape`); the panic fires at `src/lexer.rs:610` (`lex_dedent`).
when a backslash is the last character of the file, `lex_escape` consumes it and returns `Ok(())` without emitting a token.
- On an indented line, `tokenize` then calls `lex_dedent` with a token still open, and the assertion at line 610 fails. `[group("a")]` followed by a line containing just two spaces and a backslash crashes with exit 101, and so does a file holding only a tab and a backslash.
- On an unindented line the backslash just disappears: `bar: \` and `x := "a" \` both compile, and `--dump` prints them without it.

Fuzzing found about 30 distinct inputs with this one cause.

**Invalid strftime format crashes `just`**
`src/function.rs:245` and `src/function.rs:249` (`datetime`, `datetime_utc`), and `src/config.rs:137` (`Config::timestamp`).
`chrono::format(..).to_string()` panics when the format string is invalid. `x := datetime("%Q")`, `datetime_utc("%")`, `datetime_utc("%Y%")` and `just --timestamp --timestamp-format %Q foo` all abort with "a Display implementation returned an error unexpectedly" (exit 101).

**Recursive user-defined functions overflow the stack**
`src/assignment_resolver.rs:26` (functions are only checked for arity); recursion happens in `src/evaluator.rs:219` (`evaluate_defined_function`).
the resolver never follows calls into function bodies, so cycles through functions are accepted.
- `set unstable` plus `f() := f()` and `x := f()` aborts with "thread 'main' has overflowed its stack" (exit 134).
- So does the indirect cycle `f() := x` with `x := f()`.

Plain variable cycles are caught (`CircularVariableDependency`); cycles through functions are not.

**User-defined functions ignore command-line overrides**
`src/evaluator.rs:241` (`evaluate_defined_function`).
the function body is evaluated in a fresh root scope with an empty overrides map, and globals are re-evaluated from their assignments.
- With `x := "default"` and `f() := x + "!"`, running `just x=over foo` prints `default! over`: the function sees the default value while `{{x}}` sees the override.
- Backticks in globals the function uses are also re-run on every call.

**`--fmt` deletes the comment after a recipe header**
`src/parser.rs:1230` (the `expect_eol` after the dependencies), and `src/subcommand.rs:394` (the rewrite).
a comment on a recipe header line is consumed and thrown away. Running `just --unstable --fmt` on `foo x="d": # c3` rewrites the file as `foo x="d":`, so user text is lost. Comments after `set` lines and assignments are kept, so the loss is easy to miss.

**`--evaluate-format shell` puts a literal backslash before `!`**
`src/justfile.rs:321`.
`!` is backslash-escaped inside double quotes, but POSIX sh and non-interactive bash keep that backslash. With `export a-b := 'x$y"z\w`!'`, sourcing the output gives `a_b=x$y"z\w`\!`, which is wrong.

**Two modules that load the same file both get the last module's identity**
`src/compiler.rs:18` (sources already parsed are skipped), with ASTs keyed only by file path; the module path is taken from the AST at `src/analyzer.rs:314`.
with `mod a "shared.just"` and `mod b "shared.just"`, the file is parsed once under module `b`. `just a::r` prints `module_path()` as `b`, and runs with module `b`'s identity and scope. Overrides such as `a::x=1` are matched against the wrong module path (`src/analyzer.rs:199`).

## Low

**Recipe-cycle error names a recipe that is not in the cycle**
`src/recipe_resolver.rs:98`.
the code pushes `stack[0]` instead of the repeated name. For `a: b`, `b: c`, `c: b` the error says `` circular dependency `b -> c -> a` ``. The existing tests only cover cycles that start at the root.

**Mixed-whitespace check in recipe bodies never runs**
`src/lexer.rs:351`.
`char_indices().take(n).map(|(i, _)| i).next()` always gives 0, so `body_whitespace` is always empty. The `Mixed` and second `Inconsistent` branches at lines 373 and 381 can never fire. A body line starting with space then tab, after two-space indentation, is reported as "inconsistent" rather than "mix of tabs and spaces".

**SIGTERM can be forwarded to a reaped, reused PID**
`src/signal_handler.rs:131` (`spawn`) and `src/signal_handler.rs:91` (the `Terminate` handler).
the child is removed from `children` only after `f(child)` has waited on it. A SIGTERM handled in that window is sent to a PID the kernel may already have reused.

**Default chooser puts the justfile path inside shell quoting**
`src/subcommand.rs:243`.
the path is spliced into `fzf --preview '... --justfile "<path>" ...'`. A path containing `"`, `'` or `$` breaks the preview command or injects shell text.

**Clippy fails on the newer toolchain (toolchain drift)**
`src/alias.rs:17` and `src/invocation_parser.rs:333`, `:404`, `:648`.
under Rust 1.99, `clippy::pedantic` (which the project denies) now includes `manual_assert_eq` and `assert_is_empty`. These assertions fail the clippy step. The project's MSRV is 1.85.

**Rustdoc fails on regex text in lexer doc comments**
`src/lexer.rs:807`, `:824`, `:851`, `:852`.
`[a-zA-Z0-9_]`, `#[^\r\n]` and `"[^"]*"` are read as intra-doc links and footnotes, so `cargo doc --document-private-items` with `-D warnings` fails.

**Two integration tests depend on the environment**
`tests/functions.rs:88` (`env::var("USER").unwrap()`) and `tests/directories.rs:97` (needs a runtime directory, `XDG_RUNTIME_DIR` on Linux).
both fail in a clean container, so the suite is red without any code defect.

**Hygiene from the advisory lint pass**
crate-wide.

- About 90 `unwrap` warnings outside tests; examples are `src/function.rs:226`, `:448`, `:577`, `src/evaluator.rs:262`, `:274` and `src/parser.rs:1044`. They are guarded by earlier checks today, but carry no `expect` message saying why.
- 11 assertions without messages (for example `src/lexer.rs:301` and `:610`, the second being the one the backslash bug trips).
- `#[allow]` without a reason at `src/function.rs:12`, `src/lexer.rs:364` and `src/invocation_parser.rs:3`.
- Missing docs on the hidden public items `unindent`, `run`, `summary` and `request`.

The advisory pass stopped early on the clippy error, so these counts are partial. Wildcard imports (102) and print macros are deliberate in this application (the project allows `wildcard_imports`), so I did not count them.
