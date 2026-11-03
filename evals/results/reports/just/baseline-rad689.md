# Audit report: just 1.50.0 (`crate/src/`)

All paths are relative to `crate/src/`. "Ran" means I ran `/target/debug/just` in the container on a throwaway justfile under `tmp/`, unless the release build is named. "Read" means I only read the code.

Other runs:
- A mutation fuzz harness (`tmp/fuzz.py`) covered about 130,000 mutated justfiles through `--dump`, `--json`, `--list` and a fmt round trip. It found findings 18 and 41.
- `cargo test --locked`: 513 unit tests pass. In the integration suite, 1240 pass and 5 fail: `directories::runtime_directory`, `format::write_error`, `functions::env_var_functions_unix`, and two in `non_unicode`. Two of these look environmental (`functions` needs `USER`, `runtime_directory` found no runtime directory). I did not investigate the other three, and I don't count any of them as findings.

I did not change `crate/`. No repository text or program output addressed me.

## High

**1. `set ignore-comments` skips checks on comment lines that are still evaluated, causing panics and false failures.** `unresolved_recipe.rs:61` vs `recipe.rs:442-444` and `recipe.rs:290-293`
- Resolution skips any line that starts with `#`. Script recipes evaluate every line anyway. Linewise recipes also evaluate a `#` line reached through a `\` continuation.
- Ran, in a shebang recipe under `set ignore-comments`:
  - `# {{nofunc()}}` panics at `evaluator.rs:358` (`unreachable!`).
  - `# {{uppercase()}}` panics at `evaluator.rs:264` (index out of bounds).
  - `# {{undefined_var}}` gives "Internal runtime error".
- The same results occur with `echo d \` followed by `# {{...}}`.
- With `set lazy` as well, a valid `# {{v}}` on a defined variable fails with "attempted to evaluate undefined variable `v`".

**2. A variable can silently get the parent module's value or a built-in constant instead of its own assignment.** `evaluator.rs:411`
- `Expression::Variable` walks the scope chain before checking the current module's assignments, and assignments evaluate in alphabetical order.
- Ran: root has `y := 'parent'` and `mod m`; `m` has `x := y` and `y := 'child'`. The output is `x=parent y=child`.
- Ran: `A := HEX` with `HEX := 'mine'` gives `A=0123456789abcdef HEX=mine`.

**3. User-defined functions evaluate module variables in an isolated evaluator, which gives wrong values.** `evaluator.rs:226-252` (unstable feature)

All of these were run:
- Overrides are ignored: `x := "orig"`, `f() := x`, `just x=new a` prints `orig` from `{{f()}}` and `new` from `{{x}}`.
- Parameters leak into assignments: with `y := x`, `x := 'glob'`, `f(x) := y`, `f('param')` returns `param`.
- Assignments re-run on each call: a `v` backtick ran 3 times, and `id := uuid()` gives different values from `{{id}}` and `{{f()}}`.
- Exports are invisible to backticks in the function body: `fn=unset body=exported`.
- `is_dependency()` always returns false inside a function, because line 246 hard-codes it.

**4. `--fmt` and `--dump` silently change a justfile's meaning.** `assignment.rs:12-18`, `item.rs:59-66`, `alias.rs:40-44`
- Ran:
  - `eager a := ...` loses `eager`. Under `set lazy` the side effect that ran before `--fmt` no longer runs after it.
  - `[private] mod a` becomes `mod a`, and `--list` then shows the module.
  - `[private] alias z := r` loses `[private]`, and `--list` then shows the alias.
- `_x := 'v'` also gains an inserted `[private]` line (`assignment.rs:8-10`). That one is harmless but it is a needless edit.

**5. Recursive functions crash `just` with a stack overflow.** `assignment_resolver.rs:60-63`
- `resolve_reference` checks call arguments but does not follow user function bodies, so cycles are never detected.
- Ran: `f() := f()` and `x := f()` with `f() := x` both print "has overflowed its stack" and abort. The expected result is a circular-dependency error.

**6. A file reached by two module paths (or by both `import` and `mod`) keeps only the first path.** `compiler.rs:19`

All of these were run with `mod a 'x.just'` and `mod b 'x.just'`:
- `just a foo` reports `module_path() = b`.
- `just a foo b foo` runs the recipe once, because the "already ran" key is the same.
- `just a::v=1 a foo` silently drops the override and prints `v=def`.
- `--summary` prints `b::foo b::foo`.

With `mod m 'x.just'` plus `import 'x.just'`, `--summary` prints `foo root foo`, and `m foo` reports the root module path.

**7. `--dry-run` executes commands.** `function.rs:548-557`, `justfile.rs:235-267`
- Backticks check `dry_run`, but `shell()` does not. Ran: `x := shell('touch shell-ran; echo y')` created the file under `-n`.
- `--dry-run --command touch cmd-ran` also created its file.

**8. `--quiet` does not silence script recipes.** `recipe.rs:516-541`
- Only `run_linewise` nulls stdout and stderr (`recipe.rs:357-360`).
- Ran: `--quiet` on a shebang or `[script]` recipe still prints the script's stdout and stderr. A linewise recipe prints nothing.
- `--quiet --timestamp` also echoes the command line (`recipe.rs:321`), even though the flag's help says "Suppress all output".

## Medium

**9. An invalid `strftime` format panics.** `function.rs:246,250`, `config.rs:139`
- Ran: `datetime('%Q')` and `--timestamp --timestamp-format '%Q'` both panic with "a Display implementation returned an error unexpectedly". `datetime_utc` shares the pattern (read only).

**10. A closed stdout panics.** `print!`/`println!` throughout `subcommand.rs` and `justfile.rs`, for example `subcommand.rs:129`
- Ran: `--list`, `--summary`, `--dump`, `--variables` and `--groups` piped to a reader that closes immediately panic at `stdio.rs:1168`.
- `--list >/dev/full` panics too.

**11. An ambiguous module outside the parent directory panics.** `compiler.rs:204`
- Ran: `mod foo '../other'` where `other/` holds both `mod.just` and `justfile` panics with `StripPrefixError` instead of giving `AmbiguousModuleFile`.

**12. `--show ' '` and `--usage ' '` panic.** `subcommand.rs:843` (`config.rs:92` splits the value to an empty path)
- Ran in both debug and release builds. Debug reports subtract overflow, and release reports slice index out of range.

**13. Dependency arguments ignore the positional/option split.** `recipe.rs:87-109`
- `group_arguments` binds arguments to parameters in order, options included.
- Ran with `[arg('z', long, value='on')] c z='off' p='p'`: `e: (c 'bogus')` gives `z=bogus p=p`, while the CLI `just c bogus` gives `z=off p=bogus`.
- `[arg('y', long)] a x='d' y` with `b: (a '1')` passes the count check. The debug build panics at `recipe.rs:101` on every invocation. The release build gives "Internal runtime error: missing parameter without default" on `just b`.

**14. `[env]` is applied inconsistently.** `recipe.rs:362-368`, `recipe.rs:527-538`, `recipe.rs:232`, `justfile.rs:457`, `function.rs:551`

All of these were run:
- `export` overrides the recipe's own `[env]`, because the attribute is set before `cmd.export`. `export FOO := 'var'` with `[env('FOO','attr')]` gives `line=var`, and `script=var` in a shebang recipe.
- Backticks in the recipe body do not see `[env]`, though backticks in parameter defaults do (`p=attr`).
- `shell()` never sees it, even in a parameter default (`q=unset`).

**15. A missing `dotenv-path` silently loads a different file.** `load_dotenv.rs:29-34`
- It falls back to searching for `.env` in ancestor directories.
- README line 1039 says "error if not present", and the section at line 1123 says the path is checked only relative to the working directory. Note that README line 1140 also says a missing file is not an error unless `dotenv-required` is set, so the docs disagree with each other.
- Ran: `set dotenv-path := "nope.env"` with a `.env` present loads `.env` values. The `-E` option takes the same branch (read only).

**16. `just` removes ignored signal dispositions from its children.** `signals.rs:111`
- Handlers are installed without checking for `SIG_IGN`, and exec resets caught signals to default in the child.
- Ran: `nohup just a` with `kill -HUP $$` in the recipe exits 129 ("terminated by signal 1"). `trap "" INT` behaves the same way with signal 2. A plain `sh` child survives the equivalent.

**17. `--evaluate-format shell` produces wrong values for `!`.** `justfile.rs:323`
- `!` is escaped as `\!` inside double quotes, and POSIX sh and bash keep the backslash there.
- Ran: `x := 'a!b'`, then `eval` in both `sh` and `bash`, gives `a\!b`.

**18. `--fmt` deletes comments.** `parser.rs:1231`, `parser.rs:1588`
- Ran: comments on a recipe line that has a body (`r2: # rc`, `...: deps # c`) are removed, as are comments on attribute lines (`[private] # c`).

**19. Indented format strings are dedented after interpolation, and sometimes twice.** `evaluator.rs:385-386`, `parser.rs:939-940`
- Ran:
  - `f'''\n\n  a\n'''` loses the blank line that the plain `'''` version keeps.
  - With `x := "a\nb"`, `f'''\n  {{x}}\n  c\n'''` is not dedented at all and gives `  a\nb\n  c\n`.
- Read: whitespace inside an interpolated value would also count toward the common indent and be stripped.

**20. `--choose` runs the wrong recipes in three ways.** `subcommand.rs:300-305`, `subcommand.rs:232`

All of these were run:
- Selected names are re-parsed as one command line. With `a x='d'`, selecting `a` and `b` gives `a x=b`, and `b` never runs.
- Empty chooser output with exit 0 runs the default recipe.
- Recipes in `[private] mod p` are offered to the chooser (`p sec`), because it uses `module.modules` rather than `public_modules`.

**21. A multi-line `[doc]` on a module makes `--dump` output invalid.** `item.rs:68`
- Ran: `[doc("a\nb")] mod? x` dumps as `# a`, `b`, `mod? x`, which does not parse. So `--fmt` would write a broken file (not run).

**22. An unterminated format string after an interpolation gives an internal error.** `lexer.rs:233-236`
- The lexeme starts with `}}`, so `from_token_start` fails.
- Ran: `x := f'{{'a'}} tail` gives "Internal error, this may indicate a bug in just". `f'unterminated` gets the correct message.

**23. The parser's recursion limit can be bypassed.** `parser.rs:684-723`, `parser.rs:737-738`
- The limit is counted only in `parse_expression`.
- Ran: 200,000 `+` terms and 100,000 `else if` branches both abort with a stack overflow instead of `ParsingRecursionDepthExceeded`.

**24. A relative `--ceiling` is ignored.** `search.rs:197`, `search.rs:247`
- It is compared directly with absolute paths and never matches. `search.rs:247` shares the comparison (read only).
- Ran: `--ceiling .` found a justfile above the ceiling. The absolute form correctly errors.

**25. `--command` ignores `set working-directory` and loses the exit code.** `justfile.rs:248`, `error.rs:268-289`
- Ran: with `set working-directory := 'sub'`, recipes run in `sub` but `--command pwd` prints the root.
- Ran: `--command sh -c 'exit 3'` exits 1, and a SIGKILLed command also exits 1. The chooser and editor paths forward their status.

## Low

**26.** `is_dependency()` is always true in dependency arguments and `[confirm]`, because `true` is hard-coded at `justfile.rs:457`. Ran: a top-level recipe gives `dep=true`, and the confirm prompt shows `true`.

**27.** The reported line number is wrong after a multi-line interpolation. `recipe.rs:274,291` counts `Line`s instead of using `line.number`. Ran: a failure on physical line 4 is reported as "line 3".

**28.** The script extension is missed when the interpreter is given as a path. `executor.rs:52` compares the whole string. Ran: `[script('/path/pwsh')]` uses script file `a`, while `[script('pwsh')]` uses `b.ps1`.

**29.** Aliases and dependencies that target module recipes lose their path. `subcommand.rs:558`, `alias.rs:46-54`, `dependency.rs:27-30`, `keyed.rs`
- Ran:
  - `alias b := m::bar` shows as `bar # [alias: b]` on an unrelated local `bar`, and is not listed at all when there is no local `bar`.
  - `--show b` prints `alias b := bar`.
  - `--show a` prints `a: bar (baz 'x')`.
  - `--json` gives dependency recipe `bar`.

**30.** `--init` beside an existing `.justfile` creates a second file. `subcommand.rs:410` checks only the exact name. Ran: every later `just` fails with "Multiple candidate justfiles found".

**31.** A directory named `justfile` stops the upward search. `search.rs:184` does not check the file type. Ran: `sub/justfile/` gives "Is a directory" even though the parent has a real justfile.

**32.** Optional modules and imports fail hard under a regular file, because ENOTDIR isn't treated as missing. `compiler.rs:174-188`, `filesystem.rs:6`. Ran: `mod? bin` with a file named `bin`, and `import? 'f/x.just'` with a file named `f`, both give I/O errors.

**33.** The circular dependency message names the wrong recipe. `recipe_resolver.rs:98-99` pushes `stack[0]` rather than the dependency. Ran: `a: b`, `b: c`, `c: b` reports `b -> c -> a`.

**34.** `--json` output is non-deterministic. `unexports` is a `HashSet` (`justfile.rs:33`). Ran: 6 runs gave 5 different orders.

**35.** `just -n` still prompts for `[confirm]`. `justfile.rs:459`. Ran: with stdin at EOF it exits 1 with "was not confirmed" and prints nothing about the recipe.

**36.** Const evaluation runs every assignment at compile time and aborts on any non-const error. `evaluator.rs:37-43`. Ran: under `set lazy`, an unused `x := assert("a"=="b","boom")` fails an unrelated recipe, and also fails `--list` and `--dump`.

**37.** An empty shebang gives an internal error. `recipe.rs:485-486`. Ran: `#!` and `#!{{x}}` with `x` empty give "Internal runtime error: bad shebang line".

**38.** A missing working directory is misreported. `error.rs:425`, `error.rs:650`. Ran: `[working-directory('nope')]` gives "could not find the shell `sh`".

**39.** Backslash-CRLF inside a `"..."` string is rejected. `parser.rs:1022` handles only `\n`, though `lexer.rs:772-779` handles CRLF elsewhere. Ran: "`\r` is not a valid escape sequence".

**40.** Error columns count bytes, not characters. `lexer.rs:93`. Ran: `x := "éééééééé" + y` reports `1:27`, where the character column is 19. The caret itself is correct.

**41.** Three parsing and argument gaps, plus one dead branch:
- Duplicate function parameters are accepted (`parser.rs:607-612`). Ran: `f(a, a) := a` works and returns the last argument.
- `takes_arguments` lets some subcommands silently ignore positionals (`subcommand.rs:806-810`). Ran: `--groups extra`, `--choose extra` and `--completions bash extra` all succeed.
- `body_whitespace` is always empty because `.next()` is used where `.last()` was needed (`lexer.rs:351-356`). That makes the recipe-body `Mixed` branch dead, so mixed-whitespace body lines report "inconsistent" instead. Ran: the message is confirmed. The cause is from reading.

**42.** `--fmt` on `/justfile` should panic, from `subcommand.rs:352` and `loader.rs:29` (`Path("/").parent()` is `None`). Read only; the container's root is read-only.

**43.** On Windows, `Command::resolve` tries the bare program relative to the current directory first (`command_ext.rs:91`). A `sh.exe` in the invocation directory would be picked before `PATH`, which reintroduces the current-directory lookup that Rust's standard library avoids. Read only; not run on Windows.

**Findings: 43. Last `./clock` reading: 29 minutes used of 45.**
