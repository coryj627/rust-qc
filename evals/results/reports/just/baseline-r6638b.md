# Audit report: just 1.50.0 (`crate/src/`)

**How I tested.** Unless a finding says otherwise, I confirmed it by running small justfiles through `./run` with the debug binary (`cargo build --locked --bin just`). The probe scripts are in `tmp/p*.sh`.

**Other checks:**
- A differential `--fmt` fuzz (`tmp/fuzz1.py`) compared `--dump-format json` before and after formatting. It ran 2,000 generated justfiles.
- `cargo test --locked` gave 1240 passed and 5 failed: `directories::runtime_directory`, `format::write_error`, `functions::env_var_functions_unix`, and two `non_unicode::*` tests.
  - The failures I read (`env_var_functions_unix`, the two `non_unicode` tests, `write_error`) point at the container: a missing env var, a filesystem that rejects non-UTF-8 names, and a permission error that never fired. I did not read `runtime_directory`'s failure. I did not count any of the five as defects, and did not investigate them further.
- Windows-only code was only read, not run. I made no changes to `crate/`. No repository text addressed me.

## High severity: silently wrong results, data damage, crashes

1. **Variables resolve to a parent module's value or a built-in constant instead of the local definition** (`src/evaluator.rs:411`, with `src/justfile.rs:152` passing the parent module's scope down).
   - `Expression::Variable` searches the scope chain before the module's own unevaluated assignments, so the result depends on alphabetical evaluation order.
   - Root `z := 'parent'`, submodule `a := z` and `z := 'child'` prints `a=parent z=child`.
   - `A := RED` with `RED := 'mine'` gives `A` the ANSI escape, not `mine`.

2. **`--fmt` deletes `[private]` from aliases** (`src/alias.rs:40-44`). `[private]\nalias f := foo` becomes `alias f := foo`, so a hidden alias becomes public. The fuzz JSON diff shows `attributes: ["private"]` going to `[]`.

3. **`--fmt` deletes `[private]` from modules** (`src/item.rs:59-88`; the `private` field is ignored). `[private]\nmod? sub` becomes `mod? sub`.

4. **`--fmt` deletes the `eager` keyword** (`src/assignment.rs:6-17`). `eager x := ...` becomes `x := ...`, which changes when it is evaluated under lazy settings. The fuzz JSON diff shows `eager: true` going to `false`.

5. **`--fmt` breaks a shebang line, and rewrites adjacent comments** (`src/parser.rs:388`, `src/recipe.rs:617`, `src/item.rs:68`).
   - A comment directly above a recipe or module is stripped and re-emitted as `# {doc}`.
   - `#!/usr/bin/env just --justfile\nfoo:` becomes `# !/usr/bin/env just --justfile`, so the executable justfile stops working.
   - `#    aligned` becomes `# aligned`.
   - `just --list` also shows the shebang as the recipe's doc.

6. **`--fmt` corrupts or changes module docs** (`src/item.rs:67-69`).
   - `[doc("line1\nline2")]\nmod? sub` is written as `# line1\nline2\nmod? sub`. The second `--fmt` then fails with a parse error, so the file is no longer valid.
   - `# a comment\n[doc]\nmod? sub` loses `[doc]`, so the comment becomes the module's doc.

7. **Invalid `strftime` formats panic** (`src/function.rs:246` and `:250`, `src/config.rs:139-141`).
   - `.format(..).to_string()` panics on a formatter error.
   - `datetime('%Q')`, `datetime_utc('%Q')` and `--timestamp --timestamp-format %Q` all give "a Display implementation returned an error unexpectedly", exit 101.

8. **Recursive user-defined functions abort the process** (`src/assignment_resolver.rs:61-63`; calls are never followed for cycles).
   - `f(x) := f(x)` overflows the stack, exit 134.
   - So does `x := f()` with `f() := x`, which also bypasses the circular-variable check.

9. **User-defined function bodies evaluate in a fresh, wrong environment** (`src/evaluator.rs:226-252`).
   - Command-line overrides are ignored: `just x=over a` prints `over orig` for `{{x}} {{f()}}`.
   - Scoping is dynamic: `x := 'global'`, `y := x`, `f(x) := y` makes `f('param')` return `param`.
   - Exports are not visible to backticks in the body: `export A := 'val'`, `f() := `echo $A`` fails with "A: parameter not set".
   - Assignments re-run each call: the backtick behind `x` ran 3 times for `{{f()}} {{f()}} {{x}}`.
   - `is_dependency()` is always false inside a function: the run printed `true false`.

10. **`--choose` with empty selection runs the default recipe** (`src/subcommand.rs:300-305`). A chooser that exits 0 with no output leads to `justfile.run(.., &[])`. `--chooser true --choose` executed the first recipe.

11. **`--choose` multi-select merges recipes into arguments** (`src/subcommand.rs:300-305`). The output is split on whitespace and re-parsed as a command line. Selecting `a` and `b`, where `a x="dflt":`, runs `a x=b` and never runs `b`.

12. **`shell()` executes under `--dry-run`** (`src/function.rs:548-557`; backticks check `dry_run` at `src/evaluator.rs:340`, `shell` does not). `x := shell('touch marker; echo v')` with `just -n` created `marker`.

13. **`--quiet` does not silence script recipes** (`src/recipe.rs:516-541`; only `run_linewise` nulls the streams at `:357-360`). `just --quiet a` on a shebang recipe printed both its stdout and stderr.

14. **`dotenv-path` that is missing silently falls back to `.env`** (`src/load_dotenv.rs:27-41`).
   - README line 1039 says "error if not present".
   - `set dotenv-path := "missing.env"` plus `set dotenv-required` exits 0 and loads `FOO` from `./.env`.
   - `--dotenv-path nope.env` does the same.

15. **A module file used twice collapses into one module** (`src/compiler.rs:19`; ASTs are keyed by path, and `ast.module_path` comes from whichever source was parsed first).
   - With `mod a "x.just"` and `mod b "x.just"`, `just a r` prints `mod=b`.
   - `--summary` prints `b::r b::r`.
   - `just a::y=1 b r` silently drops the override.

16. **A file imported by two modules shares its variable numbers, so overrides leak** (`src/compiler.rs:19`, `src/analyzer.rs:201`, `src/evaluator.rs:186`). With `a.just` and `b.just` both importing `common.just`, `just a::x=over a r b r` also changes `b`'s `x`.

## Medium severity

17. **Variables in comment lines pass compilation, then hit an internal error** (`src/unresolved_recipe.rs:61-63`). With `set ignore-comments`, comment lines skip name resolution, but script recipes and continuation lines still evaluate them. `# {{undefined}}` in a shebang recipe, or after `echo a \`, gives "Internal runtime error ... attempted to evaluate undefined variable".

18. **`--list` misattributes or hides aliases to submodule recipes** (`src/subcommand.rs:556-561`; keyed by the target's bare name).
   - `alias f := sub::build` next to a root `build` prints `build # [alias: f]`.
   - With no root `build`, the alias does not appear at all, including under `--alias-style separate`.

19. **`--show` and JSON lose the module path of dependencies and alias targets** (`src/dependency.rs:28` and `:30`, `src/alias.rs:48-52`).
   - `test: sub::build (sub::arg "1")` shows as `test: build (arg "1")`.
   - `--show f` prints `alias f := build`.
   - JSON has `"recipe":"build"` and `"target":"build"`.

20. **The `[env]` attribute loses to exports and dotenv** (`src/recipe.rs:362-368` and `:527-538`; `.export()` runs after the attribute is set). `export FOO := 'exported'` with `[env('FOO','attr')]` prints `exported`.

21. **`[env]` reaches backticks inconsistently** (`src/recipe.rs:232`, `src/justfile.rs:457`, `src/function.rs:551`). For `[env('FOO','attr')]` with `${FOO-unset}` probes, the run gave `x=unset` for `shell()` in a parameter default, `y=attr` for a backtick in a parameter default, and `body=unset` for a backtick in the body.

22. **`is_dependency()` is wrong in dependency arguments and `[confirm]`** (`src/justfile.rs:457`; hard-coded `true`).
   - Running `b: (c is_dependency())` directly passes `true`.
   - `[confirm(is_dependency())]` prompts `true` for a top-level recipe.

23. **Unreferenced `assert()` aborts everything** (`src/evaluator.rs:37-43`; settings evaluation runs every assignment and only tolerates const errors).
   - `x := assert('a' == 'b','nope')` makes `--list` fail.
   - It also fails under `set lazy` when `x` is unused, while an unused `x := error('nope')` is fine.

24. **`--evaluate-format shell` corrupts values containing `!`** (`src/justfile.rs:323`). Inside double quotes, POSIX shells keep the backslash. `eval` of `x="hello\!"` gives `x=[hello\!]`.

25. **`--dry-run` gives false errors or prompts** (`src/evaluator.rs:340-342`). The literal `` `cmd` `` text flows into validation.
   - `[arg('x', pattern='a+')] r x=`echo a`:` fails the pattern check under `-n`.
   - ``x := assert(`echo a` == 'a','no')`` fails under `-n` but passes normally.
   - `[confirm]` still prompts under `-n` and fails on EOF (`src/justfile.rs:459`).

26. **`--fmt` drops comments** (`src/parser.rs:1230-1231` and `:1588`, via `expect_eol` at `:229`). `foo: # trailing` on a recipe with a body, and `[private] # comment`, both lose the comment. Body-less recipes keep theirs.

27. **`--fmt`, `--dump` and `--list` depend on the environment** (`src/parser.rs:957-960`). `x'...'` strings are shell-expanded at parse time. `x := x'$UNSET_VAR_ZZ'` makes `--fmt` fail with "Shell expansion failed".

28. **CRLF handling gaps.**
   - `src/lexer.rs:3`: `DEDENT_RE` does not allow `\r`, so `--fmt` on a CRLF file removes the blank line between recipes. The body swallows it.
   - `src/parser.rs:1022`: a `\`+CRLF continuation in a `"` string errors with "`\r` is not a valid escape sequence", though LF works.

29. **Parser recursion limit does not cover `+`, `/`, `&&` or `else if`** (`src/parser.rs:684-723`, `:737`).
   - A long `'a' + 'a' + ...` chain overflows the stack, exit 134.
   - The debug build crashed at n=1000 and the release build at n=10000.
   - A `/`-prefix chain and an `else if` chain also crashed (200,000 and 100,000 terms, debug build).
   - Parenthesis nesting gets the proper "Parsing recursion depth exceeded" error at 256.

30. **Relative `--ceiling` is ignored** (`src/search.rs:197`, `:247`, `src/config.rs:295`; it is compared without resolving against the invocation directory). `--ceiling ..` still found a justfile two levels up. The absolute equivalent correctly errored.

31. **Indented f-strings dedent after interpolation** (`src/evaluator.rs:385-386`). A multi-line interpolated value changes the common indent. With `v := "A\nB"`, the literal's own 4-space indent is left unstripped.

32. **`[parallel]` re-runs a dependency that already failed** (`src/justfile.rs:429-492`; the guard is only set on success). `a: b c`, with `b: d`, `c: d`, and failing `d`, printed `d-ran` twice.

33. **`--command` ignores `set working-directory`** (`src/justfile.rs:248`). Recipes run in `w/`, while `--command pwd` runs in the justfile directory.

34. **Alias to an OS-disabled recipe is a compile error** (`src/analyzer.rs:103`, `:268`). `[windows] foo:` with `alias f := foo` makes the whole justfile unusable on Linux: "Alias `f` has an unknown target `foo`".

35. **`unexport` conflicts with non-exported variables** (`src/analyzer.rs:152`). `foo := 'a'` with `unexport foo` errors "both exported and unexported", though `foo` is not exported.

36. **`--allow-missing` is ignored with `set fallback`** (`src/subcommand.rs:159-160`; the error returns before the allow-missing check). With no parent justfile, this exits 1 with "does not contain recipe". Without fallback it exits 0.

37. **Argument-count usage message omits the module path** (`src/error.rs:866`). `just sub r` with a missing argument prints `just r x`.

38. **`--show ' '` and `--usage ' '` panic** (`src/subcommand.rs:843`, via `src/config.rs:92-93`). The whitespace-only path produces an empty `Modulepath`: "attempt to subtract with overflow", exit 101.

39. **Backticks and `shell()` ignore recipe-level directories** (`src/evaluator.rs:465`). In `[working-directory("w")]`, the line runs in `w`, but `` `pwd` `` and `shell("pwd")` run in the justfile directory. `[no-cd]` backticks also use the justfile directory.
   - `absolute_path` and `path_exists` likewise ignore the attribute (`src/execution_context.rs:42-54`).

40. **Child `set export` exports the parent's non-exported variables** (`src/command_ext.rs:52`, `src/justfile.rs:152`). Root `secret := "s"` with a child `set export` gives `secret=s` in the child's recipe. README line 4158 says each module uses its own settings.

## Low severity

41. **Bare `#!` gives an internal error** (`src/parser.rs:1236` accepts it, `src/recipe.rs:485-486` fails): "Internal runtime error ... bad shebang line".

42. **A missing working directory is reported as a missing shell** (`src/recipe.rs:404-409`). A nonexistent `[working-directory]` or `set working-directory` gives "could not find the shell `sh`".

43. **`--init` creates a conflicting justfile** (`src/search.rs:124`, `src/subcommand.rs:410`). With `.justfile` present, it writes `justfile`, and later runs fail with "Multiple candidate justfiles".

44. **Circular `mod` through `..` is not detected** (`src/compiler.rs:125-128`; the path is not cleaned). `mod a "../dir/justfile"` grows the path until an I/O error. `import` reports it properly.

45. **`--edit` does not split `$EDITOR`, and an empty `$VISUAL` does not fall through** (`src/subcommand.rs:330-335`). `EDITOR='true --wait'` gives "invocation failed", and so does `VISUAL=`.

46. **`[script]` with a path to PowerShell or `cmd` gets no extension** (`src/executor.rs:52`). The full path is matched, not the file name. `[script('/x/pwsh')]` wrote `a`, while `[script('pwsh')]` wrote `b.ps1`.

47. **A relative `set tempdir` ignores the module and working directory** (`src/execution_context.rs:21`). A submodule's `set tempdir := "tmpd"` resolved against the root directory and failed, even though `sub/tmpd` existed. It also ignores `set working-directory`.

48. **`--quiet` still prints some output** (`src/recipe.rs:321`, `:242`). `--quiet --timestamp` echoes the command, and `--quiet --time` prints the timing line.

49. **`set quiet` does not silence `@` shebang recipes** (`src/recipe.rs:446`). The script body is echoed.

50. **Duplicate user-function parameters are accepted** (`src/parser.rs:607`). `f(a, a) := a` silently uses the last value.

51. **Most boolean `JUST_*` variables reject `1` and `0`** (`src/arguments.rs`; only `JUST_UNSTABLE` uses `FalseyValueParser`, line 308). `JUST_QUIET=1`, `JUST_YES=1` and `JUST_LIST_SUBMODULES=1` exit 2 with "invalid value", while `JUST_UNSTABLE=1` works.

52. **Extra arguments are silently accepted** (`src/subcommand.rs:807-816`). `--groups foo bar` and `--completions bash foo` ignore them.

53. **`--choose` includes private modules and ignores module groups** (`src/subcommand.rs:228-233`). Recipes in a `[private] mod` are offered. With `--group g`, a recipe in `[group('g')] mod m` is left out.

54. **A module's multi-line doc breaks `--list`** (`src/subcommand.rs:752-759`). It prints `sub ... # l1` followed by an unindented `l2`.

55. **`--evaluate` output is not valid just syntax when a value has `"`** (`src/justfile.rs:315`). It printed `x := "a"b"`.

56. **Attributes are stored in a sorted set, so `[metadata]` order is lost** (`src/attribute_set.rs:4`). `[metadata('z')]` then `[metadata('a')]` comes out as `a, z` in JSON.

**Number of findings: 56.** Last `./clock` reading: 17 minutes used of 45.
