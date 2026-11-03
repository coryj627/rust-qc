# Audit report: just 1.50.0 (`crate/src/`)

## Verdict

`just` is a mature application. Its short-input parser fuzzing was almost clean, and its built-in functions held up. It has real defects in `--fmt` (it rewrites source and changes its meaning), in a few panics and aborts reachable from justfile text, and in several features that interact badly: modules, user functions, `[env]`, dotenv, lazy and quiet.

Findings: 3 Blocker, 27 Major, 15 Minor, 0 Note. Total 45.

Unless a finding says otherwise, I established it by running a debug build of `just` (`cargo build --locked --bin just`) in the container with the input shown. Line numbers are in `crate/src/`. I did not change `crate/`. Harnesses ran from a throwaway copy in `tmp/fz`.

## Blocker

### `--fmt` silently changes what a justfile means
Location: assignment.rs:8-15, alias.rs:42, item.rs:35 and item.rs:64-87.
Problem:
- `eager v := 'x'` becomes `v := 'x'`.
- `[private] alias z := top` becomes `alias z := top`.
- `[private] mod a 'x.just'` becomes `mod a 'x.just'`.
- `[doc]` on a module, which suppresses the doc, is dropped, so a preceding comment becomes its doc.

After formatting, `just --list` shows `top # [alias: z]`, `a ...` and `b ... # foo doc`. None of these appeared before. The file is overwritten in place.

Established by: a corpus round-trip harness (1,111 justfiles from the repo's tests and docs, comparing the JSON dump before and after) found the alias and eager cases. Running `just --fmt` confirmed these and the module cases.

Fix: print `eager`, print alias attributes, and keep the module's private flag and doc attribute in `Item::Module`. Add a test that format preserves the dump.

### `--fmt` writes an unparseable file for a multi-line module doc
Location: item.rs:68.
Problem: `[doc('line1\nline2')]\nmod a 'x.just'` (a real newline in the string) is rewritten as `# line1`, `line2`, `mod a 'x.just'`. The next run fails with `Expected '*', ':', '$', identifier, or '+', but found end of line`. The mutation harness also found this as a re-parse failure.

Fix: print the original `[doc(...)]` attribute instead of a synthesized comment.

### `--fmt` deletes comments on recipe headers and attribute lines
Location: parser.rs:1230 (`expect_eol` discards the comment). The attribute-line comment goes the same way, though I did not trace that path to a line.
Problem: `foo: # keep this comment`, `bar: baz # another` and `[private] # why private` lose their comments when the recipe has a body. Comments after `mod`, `alias`, `set` and assignments survive.

Fix: keep the comment on the recipe or attribute and print it back.

## Major

### `datetime` and `--timestamp-format` panic on an invalid format
Location: function.rs:246, function.rs:250, config.rs:140.
Problem: `a := datetime("%Q")`, `datetime_utc("%")` and `just --timestamp --timestamp-format %Q` all give `a Display implementation returned an error unexpectedly` and exit 101.
Fix: render with `write!` into a `String` and return a function or config error on `fmt::Error`.

### A file ending in a lone backslash panics the lexer
Location: lexer.rs:780 (`lex_escape` returns Ok at EOF with an unconsumed token), which trips the assert at lexer.rs:610.
Problem: the two-byte justfile ` \` gives `assertion left == right failed` and exit 101, as does `a\n  \`. Exhaustive enumeration (2,000,719 inputs up to length 5 over 16 characters) and 600,000 seeded mutations found no other panic.
Fix: return an error when `\` is followed by EOF.

### `set ignore-comments` lets unresolved references reach the evaluator
Location: unresolved_recipe.rs:61, evaluator.rs:358.
Problem: resolution skips every comment line, but script recipes evaluate all lines, and a continued line is evaluated even when it starts with `#`.
- With a shebang body or an `echo a \` continuation, `# {{nosuch()}}` panics with `entered unreachable code`.
- `# {{undefinedvar}}` gives `Internal runtime error`.
- With `set lazy`, a defined variable in such a comment also gives an internal error.

Fix: make resolution match what execution evaluates, and skip comment lines when evaluating scripts if that is the intent.

### `--dry-run` executes `shell()`
Location: function.rs:548 (no dry-run check; the backtick path at evaluator.rs has one).
Problem: `foo:` with body `echo {{shell('touch marker-shell; echo x')}} {{`touch marker-backtick; echo y`}}` plus `y := shell('touch marker-assign; echo z')`, run with `--dry-run`, creates `marker-shell` and `marker-assign`. Backticks are correctly skipped.
Fix: apply the same dry-run handling to `shell()`.

### Two modules using one file get the wrong module identity
Location: compiler.rs:19 (a file is loaded once, keyed by path, so the AST carries the first-processed module path).
Problem: `mod a 'x.just'` and `mod b 'x.just'`: `just a::foo` prints `module=b` from `module_path()`. Overrides on `a` are also misdirected by the same mechanism (read from the code, not run).
Fix: key loaded ASTs by path and module path.

### Overrides leak between modules that import the same file
Location: analyzer.rs:201 (overrides are keyed by `Number`, shared through the single AST).
Problem: `mod a` and `mod b` each `import '../common.just'`. `just a::v=1 b::show` prints `module=b v=1`.
Fix: number assignments per module instantiation, or key overrides by module and name.

### Printing to a closed or full stdout or stderr panics
Location: the print macros throughout subcommand.rs, justfile.rs and recipe.rs (for example recipe.rs:337).
Problem:
- `just --list | true`, and likewise `--summary`, `--variables`, `--dump`, `--groups`, `--evaluate`, `--completions` and `--show`, panic with `failed printing to stdout: Broken pipe` and exit 101.
- `--list >/dev/full` also panics.
- Running a recipe with `2>/dev/full`, or with stderr closed by a reader, panics while echoing the command.

`--man` already handles this correctly with `Error::StdoutIo`.
Fix: use `writeln!` with errors returned, treating `BrokenPipe` as a quiet exit.

### Long expression chains overflow the stack instead of hitting the recursion limit
Location: parser.rs:656 (only `parse_expression` counts depth), parser.rs:684, 701 and 726.
Problem: `x := 'a' + 'a' + ...` aborts with `has overflowed its stack` and exit 134 at 20,000 terms in a non-LTO release build (5,000 in debug). `&&` and `else if` chains behave the same. The parser has a depth limit with a proper error, but these paths bypass it.
Fix: count depth in the recursive arms, or parse the chains iteratively.

### Long dependency and variable chains overflow the stack
Location: recipe_resolver.rs:39-117, assignment_resolver.rs:35-105.
Problem: 5,000 chained recipes (`r0: r1`, `r1: r2`, ...) or 5,000 chained assignments abort with exit 134 in the release build.
Fix: use an explicit stack, or return a depth error.

### User-defined functions are not checked for recursion
Location: assignment_resolver.rs:61-62 (calls are not followed), evaluator.rs:354.
Problem: `f(x) := f(x)` aborts with exit 134. `a := f()` with `f() := a` also aborts, and the circular-variable check does not see it.
Fix: follow calls during resolution and report a cycle.

### User function bodies lose overrides and re-evaluate variables
Location: evaluator.rs:226 and 241 (fresh root scope, empty overrides).
Problem:
- `x := 'default'`, `f() := x`, `just x=1` prints `x=1 f=default`.
- `x := `echo run >&2; echo v`` used once directly and twice through `f()` runs the backtick three times.

Fix: evaluate the body with a scope chained to the module scope, with parameters in a child.

### User function bodies lose exports and `is_dependency`
Location: evaluator.rs:226, plus `export` at command_ext.rs (a root scope has no parent, so nothing is exported or removed).
Problem: with `export FOO := 'bar'`, `f() := `echo fb:$FOO`` fails with `FOO: parameter not set`, while the same backtick in a recipe works. `is_dependency` is hard-coded false in the function evaluator (read from the code, not run).
Fix: the same scope fix, and pass `is_dependency` through.

### `--quiet` does not silence script recipes
Location: recipe.rs:527-541 (`run_linewise` nulls stdio at recipe.rs:357; `run_script` does not).
Problem: help says "Suppress all output", but `just --quiet foo` on a shebang recipe prints the script's stdout and stderr. A linewise recipe stays silent.
Fix: null stdout and stderr in `run_script` too.

### `[env]` is overridden by exports and dotenv
Location: recipe.rs:362-368 (and recipe.rs:527-533): `[env]` is applied before `cmd.export`.
Problem: `export X := 'var'` with `[env('X','attr')] foo: echo $X` prints `var`. With `set dotenv-load` and `X=dot` in `.env` it prints `dot`. The backtick path applies `env` last, so the two paths disagree.
Fix: apply `[env]` after `export`.

### `[env]` does not reach backticks or `shell()` in the recipe body
Location: recipe.rs:232.
Problem: `[env('A','attr')] foo: echo {{`echo A=$A`}}` fails with `A: parameter not set`, as does `shell('echo A=$A')`. A backtick in a parameter default on the same recipe sees `A=attr`.
Fix: build the body evaluator with the recipe's env, as `evaluate_parameters` does.

### A missing dotenv path silently loads a different file
Location: load_dotenv.rs:27-34.
Problem:
- `set dotenv-path := 'missing.env'` loads `.env` from the ancestors and prints `X=from_dotenv`. The settings table says "error if not present", and the prose says it is checked only at that path.
- Worse, `just -E .env.prod m::show` prints `X=default` while root recipes get `X=prod`. The path is joined to the module directory, misses, and the fallback `.env` overrides the parent's values.

Fix: return after a `dotenv-path` miss (an error if required), and resolve `-E` against one fixed base.

### Indented format strings stop unindenting when a value has lines
Location: evaluator.rs:385.
Problem: unindent runs after interpolation. With `x := "a\nb"` and `f'''` holding the lines `foo`, `{{x}}`, `bar` indented 4 spaces, the output is `    foo`, `    a`, `b`, `    bar`. The literal's indentation should be removed regardless of the value.
Fix: compute the indent from the literal parts only.

### `set lazy` in a module is ignored
Location: justfile.rs:187 (only the root's setting is read).
Problem: with a non-lazy root and `set lazy` in `mod m`, an unused `x := `exit 4`` in the module makes `just m::foo` fail. Conversely, a root `set lazy` also skips variables in a module that never asked for it.
Fix: decide per module in `evaluate_scopes`.

### `set lazy` takes exponential time on diamond dependencies
Location: justfile.rs:196 (no visited set).
Problem: chains of `aN: bN cN`, `bN: aN+1`, `cN: aN+1` took 0.46s, 1.83s and 7.88s at 20, 22 and 24 levels, and more than 20s at 40. The same 40-level file without `set lazy` finishes immediately.
Fix: track visited recipes.

### `--evaluate-format shell` corrupts values containing `!`
Location: justfile.rs:323.
Problem: `x := 'a!b'` is emitted as `x="a\!b"`. POSIX sh and bash keep the backslash inside double quotes, so sourcing it gives `a\!b` in both.
Fix: do not escape `!`, or use single quotes.

### Alias and dependency targets in a submodule are misreported
Location: subcommand.rs:558, dependency.rs:28 and 30, alias.rs (Display), keyed.rs.
Problem: everything uses the bare recipe name. With `mod m`, `alias x := m::bar` and a local `bar`:
- `--list` shows the alias on the local recipe (`bar # doc for local bar [alias: x]`), while `just x` runs `m::bar`.
- Without a local `bar`, the alias is not listed at all.
- `--show foo` for `foo: m::bar` prints `foo: bar`.
- `--show x` prints `alias x := foo` for `m::foo`.
- `--json` gives `recipe: 'bar'` and `target: 'bar'`.

Fix: carry the namepath through display, serialization and alias grouping.

### Failure line numbers are wrong when the header spans lines
Location: recipe.rs:274.
Problem: `foo: a \`, `  b`, `  echo 1`, `  exit 3` reports `failed on line 3`, but `exit 3` is on line 4. The count starts from the recipe name's line.
Fix: use `Line::number`, which the parser already records.

### `--choose` offers recipes from private modules
Location: subcommand.rs:232.
Problem: `[private] mod hidden` is hidden by `--list` and `--summary`, but the chooser receives `hidden bar`.
Fix: use `public_modules`.

### `--choose` runs the default recipe when nothing is selected
Location: subcommand.rs:300-305.
Problem: a chooser that exits 0 with empty output, such as `--chooser 'cat > /dev/null'`, runs the default recipe (`DEFAULT-RAN`).
Fix: return without running anything when the selection is empty.

### `--ceiling` is escaped by `set fallback`
Location: search.rs:90-99.
Problem: from `nest/x/y` with `--ceiling /…/nest/x`, a `set fallback` in `nest/x/justfile` still runs `root` from `nest/justfile`. Help says not to ascend above the ceiling.
Fix: stop at the ceiling in `search_parent_directory`.

### A relative `--ceiling` is never honored
Location: search.rs:197 and 247 (raw path comparison against absolute ancestors).
Problem: from `c1/c2/c3`, `--ceiling . r` walks up and runs a recipe from a higher justfile, while the absolute form correctly gives `No justfile found`.
Fix: resolve the ceiling against the invocation directory.

### Circular dependency errors name the wrong cycle
Location: recipe_resolver.rs:98-99.
Problem: `a: b`, `b: c`, `c: b` reports `b -> c -> a`. The cycle is `b -> c -> b`, and `a` is not part of it. The code pushes `stack[0]` rather than the repeated name.
Fix: push the dependency name.

### Valid dependency arguments crash on option parameters
Location: recipe.rs:101, unresolved_recipe.rs (argument-count check).
Problem: `[arg('o', long)] foo a="1" o:` with `bar: (foo "x")` passes the count check, then fails. Debug builds panic with `assertion failed: parameter.default.is_some()`. Release builds give `Internal runtime error ... missing parameter without default`.
Fix: reject this at compile time with a proper count error.

## Minor

### `set lazy` still fails on unused constant asserts and regexes
Location: evaluator.rs:39-40.
Problem: constant evaluation runs at compile time for every command. `x := assert("a" == "b", "no")` or `y := if "a" =~ "(" {...}` aborts `just foo` under `set lazy`, even though neither variable is used. I ran only the plain-run form. Since this happens during compilation, other compiling subcommands such as `--list` should be affected too, but I did not run them.
Fix: treat these errors as deferred, as `Const` errors are.

### Constant evaluation can exhaust memory on `--list`
Location: evaluator.rs:39.
Problem: 35 levels of `aN := aN+1 + aN+1` killed `just --list` for memory (exit 137 under the container's 2.5 GB limit, needing about 256 GB).
Fix: skip constant evaluation of assignments that no setting references, or cap the size.

### An invoked recipe's run is blocked by unrelated module variables
Location: justfile.rs:146.
Problem: with `mod m` holding `x := `exit 5``, `just top` (a root recipe) fails with exit 5 and runs the backtick.
Fix: evaluate module scopes only when needed.

### Parallel dependencies rerun a failed recipe
Location: justfile.rs run_recipe (`*guard` is set only on success).
Problem: `[parallel] foo: a b`, with `a: c`, `b: c` and `c` failing, prints `C-RAN` twice.
Fix: record completion, including failure.

### `--quiet` still prints with `--time`, `--timestamp` and `--explain`
Location: recipe.rs:254, 321, 228.
Problem: `just --quiet --timestamp foo` prints `[08:20:48] echo hi`, `--time` prints the completion line, and `--explain` prints the doc.
Fix: gate these on `verbosity.quiet()`.

### Backticks ignore the recipe's working directory
Location: evaluator.rs:465.
Problem: in `[working-directory('sub')]` or `[no-cd]` recipes, `{{`pwd`}}`, `shell('pwd')` and `path_exists` use the justfile directory while `$(pwd)` uses the recipe's, so one line prints two different directories. The setting's docs promise "recipes and backticks".
Fix: pass the recipe's directory to the body evaluator.

### Child `set export` exports the parent's non-exported variables
Location: command_ext.rs:52.
Problem: `secret := 'parentvalue'` in the root plus `set export` in a module makes `$secret` visible in module recipes. The child's setting is applied to ancestor scopes.
Fix: use the export flag stored on each binding.

### Alias or dependency on a platform-disabled recipe breaks the whole justfile
Location: analyzer.rs:103.
Problem: `alias b := build` with `[windows] build:` fails on Linux for unrelated recipes such as `just other`, with `Alias b has an unknown target`. `all: build` fails the same way, and that case was seen under `--list`. Aliases cannot carry platform attributes.
Fix: allow them to resolve to disabled recipes, failing only if invoked.

### Multi-line module doc breaks `--list`
Location: subcommand.rs:755.
Problem: the list prints `m ... # first`, then `second` at column 0. Recipes handle this case.
Fix: reuse the multi-line recipe path.

### Default chooser breaks on a quote in the path
Location: subcommand.rs:243-244.
Problem: in a directory named `it's`, `just --choose` fails with `Unterminated quoted string`. The path is spliced unescaped into a shell string.
Fix: quote it, or pass it through the environment.

### `$EDITOR` with arguments fails
Location: subcommand.rs:334.
Problem: `EDITOR='echo edit-with-args -x' just --edit` fails with `invocation failed: No such file or directory`, so `code --wait` cannot work.
Fix: split the value or run it through the shell.

### Circular module detection misses unclean paths
Location: compiler.rs:125-128.
Problem: `mod a 'sub/../circmod'` is not detected as circular. It grows the path `sub/../sub/../...` until an I/O path error, instead of `is circular`. `import` is cleaned and reports correctly.
Fix: lexiclean `full`.

### CRLF justfiles lose blank lines when formatted
Location: lexer.rs:3.
Problem: `DEDENT_RE` does not recognize `\r\n`, so the blank line between recipes becomes part of the body and is dropped. `--fmt` output has no separator. LF files keep it.
Fix: allow `\r` in the regex.

### Option defaults reject a following required positional
Location: analyzer.rs:371.
Problem: `[arg("o", long)] foo o="d" p:` fails with `Non-default parameter p follows default parameter`, although `--o` is unambiguous on the command line.
Fix: do not set `passed_default` for options, once the dependency-argument crash above is fixed.

### Lower-priority code issues found by reading, without a demonstration
- A signal can reach a reused PID (signal_handler.rs:100, 135, 139). The child is reaped in `f(child)` before it leaves `children`, so a SIGTERM in that window is sent to a stale PID. Fix: remove the child under the lock before reaping, or use `try_wait`.
- `body_whitespace` is always empty (lexer.rs:351; `.next()` always yields index 0), so the recipe-body `Mixed` branch is dead. Input ` \t` after a two-space body reports Inconsistent, as I confirmed.
- `Config::new().unwrap()` in the public `just::summary::summary` panics if the current directory is gone (summary.rs:31).
- The fuzz target does not compile (fuzzing.rs:4, lib.rs:174-176). `RUSTFLAGS="--cfg fuzzing" cargo check --lib` fails with E0433 because `testing` is `cfg(test)`. `testing::compile` also panics on any invalid input.
- Hygiene:
  - Missing docs on `Arguments`, `INIT_JUSTFILE`, `unindent` and the crate.
  - 7 `#[allow]` attributes in `crate/src/` (8 lint hits across the workspace), none with a reason.
  - 11 asserts without messages.
  - 3 `map_err(|_|` discarding the error.
  - Rustdoc fails with private items on `[a-zA-Z_]...` comments (lexer.rs:807, 824, 851-852).
  - Clippy fails only on the newer toolchain here (`manual_assert_eq` at alias.rs:17). That is toolchain drift, since CI pins older tools.
  - Two tests assume the environment: `runtime_directory` needs `XDG_RUNTIME_DIR` and `env_var_functions_unix` needs `USER`.

## Verification

- `cargo fmt --all --check`: pass.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: fail, toolchain drift only (see above).
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --all-features`: fail, comment link syntax (see above).
- `cargo test --workspace --all-features`: 513 unit tests pass. Integration: 1240 pass, 5 fail, 8 ignored. I attribute all 5 failures to this environment, not to code defects:
  - `runtime_directory`: `XDG_RUNTIME_DIR` is unset.
  - `env_var_functions_unix`: `USER` is unset.
  - Two `non_unicode` tests: the mount rejects non-UTF-8 names (os error 84).
  - `format::write_error`: expected a permission error, but the write succeeded on this mount. This one is my inference.
- `RUSTFLAGS="--cfg fuzzing" cargo check --lib`: fail (see above).
- `cargo +nightly-2026-10-01 miri test`: not run. The only `unsafe` is three small signal-setup blocks (signals.rs:24, 57, 117), each with a SAFETY comment. No test exercises them under Miri, so I reviewed them by reading and found no problem. The handler itself only calls `write` and `abort`.
- Not exercised: Windows code paths, and MSRV 1.85 (toolchain unavailable).

No text in the repository or tool output addressed me as instructions.

45 findings. Last clock reading: 25 minutes used of 45.
