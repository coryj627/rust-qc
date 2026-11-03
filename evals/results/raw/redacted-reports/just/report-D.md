# Audit report: just 1.50.0 (`crate/src/`)

## Summary

`just` is a mature application. Its short-input parser fuzzing was almost clean, and its built-in functions held up. It has real defects in `--fmt` (it rewrites source and changes its meaning), in a few panics and aborts reachable from justfile text, and in several features that interact badly: modules, user functions, `[env]`, dotenv, lazy and quiet.

 Total 45.

Unless a finding says otherwise, I established it by running a debug build of `just` (`cargo build --locked --bin just`) in the container with the input shown. Line numbers are in `crate/src/`. I did not change `crate/`. Harnesses ran from a throwaway copy in `tmp/fz`.

## High

**`--fmt` silently changes what a justfile means**
assignment.rs:8-15, alias.rs:42, item.rs:35 and item.rs:64-87.

- `eager v := 'x'` becomes `v := 'x'`.
- `[private] alias z := top` becomes `alias z := top`.
- `[private] mod a 'x.just'` becomes `mod a 'x.just'`.
- `[doc]` on a module, which suppresses the doc, is dropped, so a preceding comment becomes its doc.

After formatting, `just --list` shows `top # [alias: z]`, `a ...` and `b ... # foo doc`. None of these appeared before. The file is overwritten in place.

a corpus round-trip harness (1,111 justfiles from the repo's tests and docs, comparing the JSON dump before and after) found the alias and eager cases. Running `just --fmt` confirmed these and the module cases.

**`--fmt` writes an unparseable file for a multi-line module doc**
item.rs:68.
`[doc('line1\nline2')]\nmod a 'x.just'` (a real newline in the string) is rewritten as `# line1`, `line2`, `mod a 'x.just'`. The next run fails with `Expected '*', ':', '$', identifier, or '+', but found end of line`. The mutation harness also found this as a re-parse failure.

**`--fmt` deletes comments on recipe headers and attribute lines**
parser.rs:1230 (`expect_eol` discards the comment). The attribute-line comment goes the same way, though I did not trace that path to a line.
`foo: # keep this comment`, `bar: baz # another` and `[private] # why private` lose their comments when the recipe has a body. Comments after `mod`, `alias`, `set` and assignments survive.

## Medium

**`datetime` and `--timestamp-format` panic on an invalid format**
function.rs:246, function.rs:250, config.rs:140.
`a := datetime("%Q")`, `datetime_utc("%")` and `just --timestamp --timestamp-format %Q` all give `a Display implementation returned an error unexpectedly` and exit 101.

**A file ending in a lone backslash panics the lexer**
lexer.rs:780 (`lex_escape` returns Ok at EOF with an unconsumed token), which trips the assert at lexer.rs:610.
the two-byte justfile ` \` gives `assertion left == right failed` and exit 101, as does `a\n  \`. Exhaustive enumeration (2,000,719 inputs up to length 5 over 16 characters) and 600,000 seeded mutations found no other panic.

**`set ignore-comments` lets unresolved references reach the evaluator**
unresolved_recipe.rs:61, evaluator.rs:358.
resolution skips every comment line, but script recipes evaluate all lines, and a continued line is evaluated even when it starts with `#`.
- With a shebang body or an `echo a \` continuation, `# {{nosuch()}}` panics with `entered unreachable code`.
- `# {{undefinedvar}}` gives `Internal runtime error`.
- With `set lazy`, a defined variable in such a comment also gives an internal error.

**`--dry-run` executes `shell()`**
function.rs:548 (no dry-run check; the backtick path at evaluator.rs has one).
`foo:` with body `echo {{shell('touch marker-shell; echo x')}} {{`touch marker-backtick; echo y`}}` plus `y := shell('touch marker-assign; echo z')`, run with `--dry-run`, creates `marker-shell` and `marker-assign`. Backticks are correctly skipped.

**Two modules using one file get the wrong module identity**
compiler.rs:19 (a file is loaded once, keyed by path, so the AST carries the first-processed module path).
`mod a 'x.just'` and `mod b 'x.just'`: `just a::foo` prints `module=b` from `module_path()`. Overrides on `a` are also misdirected by the same mechanism (read from the code, not run).

**Overrides leak between modules that import the same file**
analyzer.rs:201 (overrides are keyed by `Number`, shared through the single AST).
`mod a` and `mod b` each `import '../common.just'`. `just a::v=1 b::show` prints `module=b v=1`.

**Printing to a closed or full stdout or stderr panics**
the print macros throughout subcommand.rs, justfile.rs and recipe.rs (for example recipe.rs:337).

- `just --list | true`, and likewise `--summary`, `--variables`, `--dump`, `--groups`, `--evaluate`, `--completions` and `--show`, panic with `failed printing to stdout: Broken pipe` and exit 101.
- `--list >/dev/full` also panics.
- Running a recipe with `2>/dev/full`, or with stderr closed by a reader, panics while echoing the command.

`--man` already handles this correctly with `Error::StdoutIo`.

**Long expression chains overflow the stack instead of hitting the recursion limit**
parser.rs:656 (only `parse_expression` counts depth), parser.rs:684, 701 and 726.
`x := 'a' + 'a' + ...` aborts with `has overflowed its stack` and exit 134 at 20,000 terms in a non-LTO release build (5,000 in debug). `&&` and `else if` chains behave the same. The parser has a depth limit with a proper error, but these paths bypass it.

**Long dependency and variable chains overflow the stack**
recipe_resolver.rs:39-117, assignment_resolver.rs:35-105.
5,000 chained recipes (`r0: r1`, `r1: r2`, ...) or 5,000 chained assignments abort with exit 134 in the release build.

**User-defined functions are not checked for recursion**
assignment_resolver.rs:61-62 (calls are not followed), evaluator.rs:354.
`f(x) := f(x)` aborts with exit 134. `a := f()` with `f() := a` also aborts, and the circular-variable check does not see it.

**User function bodies lose overrides and re-evaluate variables**
evaluator.rs:226 and 241 (fresh root scope, empty overrides).

- `x := 'default'`, `f() := x`, `just x=1` prints `x=1 f=default`.
- `x := `echo run >&2; echo v`` used once directly and twice through `f()` runs the backtick three times.

**User function bodies lose exports and `is_dependency`**
evaluator.rs:226, plus `export` at command_ext.rs (a root scope has no parent, so nothing is exported or removed).
with `export FOO := 'bar'`, `f() := `echo fb:$FOO`` fails with `FOO: parameter not set`, while the same backtick in a recipe works. `is_dependency` is hard-coded false in the function evaluator (read from the code, not run).

**`--quiet` does not silence script recipes**
recipe.rs:527-541 (`run_linewise` nulls stdio at recipe.rs:357; `run_script` does not).
help says "Suppress all output", but `just --quiet foo` on a shebang recipe prints the script's stdout and stderr. A linewise recipe stays silent.

**`[env]` is overridden by exports and dotenv**
recipe.rs:362-368 (and recipe.rs:527-533): `[env]` is applied before `cmd.export`.
`export X := 'var'` with `[env('X','attr')] foo: echo $X` prints `var`. With `set dotenv-load` and `X=dot` in `.env` it prints `dot`. The backtick path applies `env` last, so the two paths disagree.

**`[env]` does not reach backticks or `shell()` in the recipe body**
recipe.rs:232.
`[env('A','attr')] foo: echo {{`echo A=$A`}}` fails with `A: parameter not set`, as does `shell('echo A=$A')`. A backtick in a parameter default on the same recipe sees `A=attr`.

**A missing dotenv path silently loads a different file**
load_dotenv.rs:27-34.

- `set dotenv-path := 'missing.env'` loads `.env` from the ancestors and prints `X=from_dotenv`. The settings table says "error if not present", and the prose says it is checked only at that path.
- Worse, `just -E .env.prod m::show` prints `X=default` while root recipes get `X=prod`. The path is joined to the module directory, misses, and the fallback `.env` overrides the parent's values.

**Indented format strings stop unindenting when a value has lines**
evaluator.rs:385.
unindent runs after interpolation. With `x := "a\nb"` and `f'''` holding the lines `foo`, `{{x}}`, `bar` indented 4 spaces, the output is `    foo`, `    a`, `b`, `    bar`. The literal's indentation should be removed regardless of the value.

**`set lazy` in a module is ignored**
justfile.rs:187 (only the root's setting is read).
with a non-lazy root and `set lazy` in `mod m`, an unused `x := `exit 4`` in the module makes `just m::foo` fail. Conversely, a root `set lazy` also skips variables in a module that never asked for it.

**`set lazy` takes exponential time on diamond dependencies**
justfile.rs:196 (no visited set).
chains of `aN: bN cN`, `bN: aN+1`, `cN: aN+1` took 0.46s, 1.83s and 7.88s at 20, 22 and 24 levels, and more than 20s at 40. The same 40-level file without `set lazy` finishes immediately.

**`--evaluate-format shell` corrupts values containing `!`**
justfile.rs:323.
`x := 'a!b'` is emitted as `x="a\!b"`. POSIX sh and bash keep the backslash inside double quotes, so sourcing it gives `a\!b` in both.

**Alias and dependency targets in a submodule are misreported**
subcommand.rs:558, dependency.rs:28 and 30, alias.rs (Display), keyed.rs.
everything uses the bare recipe name. With `mod m`, `alias x := m::bar` and a local `bar`:
- `--list` shows the alias on the local recipe (`bar # doc for local bar [alias: x]`), while `just x` runs `m::bar`.
- Without a local `bar`, the alias is not listed at all.
- `--show foo` for `foo: m::bar` prints `foo: bar`.
- `--show x` prints `alias x := foo` for `m::foo`.
- `--json` gives `recipe: 'bar'` and `target: 'bar'`.

**Failure line numbers are wrong when the header spans lines**
recipe.rs:274.
`foo: a \`, `  b`, `  echo 1`, `  exit 3` reports `failed on line 3`, but `exit 3` is on line 4. The count starts from the recipe name's line.

**`--choose` offers recipes from private modules**
subcommand.rs:232.
`[private] mod hidden` is hidden by `--list` and `--summary`, but the chooser receives `hidden bar`.

**`--choose` runs the default recipe when nothing is selected**
subcommand.rs:300-305.
a chooser that exits 0 with empty output, such as `--chooser 'cat > /dev/null'`, runs the default recipe (`DEFAULT-RAN`).

**`--ceiling` is escaped by `set fallback`**
search.rs:90-99.
from `nest/x/y` with `--ceiling /…/nest/x`, a `set fallback` in `nest/x/justfile` still runs `root` from `nest/justfile`. Help says not to ascend above the ceiling.

**A relative `--ceiling` is never honored**
search.rs:197 and 247 (raw path comparison against absolute ancestors).
from `c1/c2/c3`, `--ceiling . r` walks up and runs a recipe from a higher justfile, while the absolute form correctly gives `No justfile found`.

**Circular dependency errors name the wrong cycle**
recipe_resolver.rs:98-99.
`a: b`, `b: c`, `c: b` reports `b -> c -> a`. The cycle is `b -> c -> b`, and `a` is not part of it. The code pushes `stack[0]` rather than the repeated name.

**Valid dependency arguments crash on option parameters**
recipe.rs:101, unresolved_recipe.rs (argument-count check).
`[arg('o', long)] foo a="1" o:` with `bar: (foo "x")` passes the count check, then fails. Debug builds panic with `assertion failed: parameter.default.is_some()`. Release builds give `Internal runtime error ... missing parameter without default`.

## Low

**`set lazy` still fails on unused constant asserts and regexes**
evaluator.rs:39-40.
constant evaluation runs at compile time for every command. `x := assert("a" == "b", "no")` or `y := if "a" =~ "(" {...}` aborts `just foo` under `set lazy`, even though neither variable is used. I ran only the plain-run form. Since this happens during compilation, other compiling subcommands such as `--list` should be affected too, but I did not run them.

**Constant evaluation can exhaust memory on `--list`**
evaluator.rs:39.
35 levels of `aN := aN+1 + aN+1` killed `just --list` for memory (exit 137 under the container's 2.5 GB limit, needing about 256 GB).

**An invoked recipe's run is blocked by unrelated module variables**
justfile.rs:146.
with `mod m` holding `x := `exit 5``, `just top` (a root recipe) fails with exit 5 and runs the backtick.

**Parallel dependencies rerun a failed recipe**
justfile.rs run_recipe (`*guard` is set only on success).
`[parallel] foo: a b`, with `a: c`, `b: c` and `c` failing, prints `C-RAN` twice.

**`--quiet` still prints with `--time`, `--timestamp` and `--explain`**
recipe.rs:254, 321, 228.
`just --quiet --timestamp foo` prints `[08:20:48] echo hi`, `--time` prints the completion line, and `--explain` prints the doc.

**Backticks ignore the recipe's working directory**
evaluator.rs:465.
in `[working-directory('sub')]` or `[no-cd]` recipes, `{{`pwd`}}`, `shell('pwd')` and `path_exists` use the justfile directory while `$(pwd)` uses the recipe's, so one line prints two different directories. The setting's docs promise "recipes and backticks".

**Child `set export` exports the parent's non-exported variables**
command_ext.rs:52.
`secret := 'parentvalue'` in the root plus `set export` in a module makes `$secret` visible in module recipes. The child's setting is applied to ancestor scopes.

**Alias or dependency on a platform-disabled recipe breaks the whole justfile**
analyzer.rs:103.
`alias b := build` with `[windows] build:` fails on Linux for unrelated recipes such as `just other`, with `Alias b has an unknown target`. `all: build` fails the same way, and that case was seen under `--list`. Aliases cannot carry platform attributes.

**Multi-line module doc breaks `--list`**
subcommand.rs:755.
the list prints `m ... # first`, then `second` at column 0. Recipes handle this case.

**Default chooser breaks on a quote in the path**
subcommand.rs:243-244.
in a directory named `it's`, `just --choose` fails with `Unterminated quoted string`. The path is spliced unescaped into a shell string.

**`$EDITOR` with arguments fails**
subcommand.rs:334.
`EDITOR='echo edit-with-args -x' just --edit` fails with `invocation failed: No such file or directory`, so `code --wait` cannot work.

**Circular module detection misses unclean paths**
compiler.rs:125-128.
`mod a 'sub/../circmod'` is not detected as circular. It grows the path `sub/../sub/../...` until an I/O path error, instead of `is circular`. `import` is cleaned and reports correctly.

**CRLF justfiles lose blank lines when formatted**
lexer.rs:3.
`DEDENT_RE` does not recognize `\r\n`, so the blank line between recipes becomes part of the body and is dropped. `--fmt` output has no separator. LF files keep it.

**Option defaults reject a following required positional**
analyzer.rs:371.
`[arg("o", long)] foo o="d" p:` fails with `Non-default parameter p follows default parameter`, although `--o` is unambiguous on the command line.

**Lower-priority code issues found by reading, without a demonstration**
- A signal can reach a reused PID (signal_handler.rs:100, 135, 139). The child is reaped in `f(child)` before it leaves `children`, so a SIGTERM in that window is sent to a stale PID.
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
