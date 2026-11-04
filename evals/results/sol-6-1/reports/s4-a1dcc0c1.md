Found 12 reproducible problems. The crate was unchanged; probes used `/work/tmp`.

1. **[P1] `shell()` executes during `--dry-run`.**  
   [function.rs:548](/work/crate/src/function.rs:548) calls the shell without checking `dry_run`. A recipe containing `echo {{shell('touch marker; echo hi')}}` creates `marker` under `just --dry-run`. Backticks correctly suppress execution.

2. **[P2] Reusing a file as two modules merges their identities.**  
   [compiler.rs:18](/work/crate/src/compiler.rs:18) caches parsed files solely by filesystem path, although their ASTs contain module paths. With `mod a 'shared.just'` and `mod b 'shared.just'`, running `a::hello b::hello` executes only once, with `module_path()` returning `b`. Module-specific overrides also collide.

3. **[P2] Parent bindings override unevaluated local assignments.**  
   [evaluator.rs:409](/work/crate/src/evaluator.rs:409) searches inherited scope before local assignments. With root `y := 'parent'` and module assignments `x := y`, `y := 'child'`, a module recipe prints `parent:child` for `{{x}}:{{y}}`. Evaluating `m::x` separately returns `child`, making results depend on evaluation mode.

4. **[P2] `--fmt` removes module privacy.**  
   [item.rs:59](/work/crate/src/item.rs:59) ignores the module’s `private` field. Formatting `[private]` followed by `mod secrets` removes the attribute, exposing the module through listing and completion.

5. **[P2] Formatting multiline module documentation produces invalid source.**  
   [item.rs:67](/work/crate/src/item.rs:67) writes documentation with only one comment prefix. A valid `[doc('first` newline `second')]` becomes `# first` followed by bare `second`. `--fmt` succeeds, but subsequent parsing fails.

6. **[P2] `--fmt` removes `eager`.**  
   [assignment.rs:7](/work/crate/src/assignment.rs:7) serializes `private` and `export`, but omits `eager`. With `set lazy`, formatting an eager assignment silently stops its evaluation when otherwise unused. This affects the unstable eager-assignment feature.

7. **[P2] User-defined functions discard overrides and cached assignment values.**  
   [evaluator.rs:226](/work/crate/src/evaluator.rs:226) creates a fresh root scope and empty override map for every call. For assignments `x := 'default'`, `f() := x`, `a := f()`, running `just --unstable --set x override --evaluate a` returns `default`. Referencing an assignment containing `uuid()` through repeated function calls also generates different values.

8. **[P2] Cycles through user-defined functions abort the process.**  
   [assignment_resolver.rs:60](/work/crate/src/assignment_resolver.rs:60) validates call arity without traversing function dependencies. `f() := x` together with `x := f()` passes analysis, then `just --unstable --evaluate x` aborts with a stack overflow instead of reporting a dependency cycle.

9. **[P2] Lazy dependency collection grows exponentially.**  
   [justfile.rs:196](/work/crate/src/justfile.rs:196) traverses dependencies without recording visited recipes. A graph with `r0:`, `r1: r0`, and each `rN: rN-1 rN-2` repeatedly traverses shared dependencies. With 36 recipes, execution exceeded a two-second timeout under `set lazy`; without it, execution finished in roughly 12 ms.

10. **[P2] Parser recursion checks miss operator chains.**  
    [parser.rs:701](/work/crate/src/parser.rs:701) recursively parses concatenations and joins without updating the recursion counter. An expression containing 2,000 `'a'` operands joined by `+` aborts with a stack overflow. A malformed chain also crashes `--fmt --check`, confirming the failure occurs during parsing.

11. **[P2] Invalid date formats cause panics.**  
    [function.rs:245](/work/crate/src/function.rs:245) converts Chrono’s fallible formatting output using `to_string()`. Both `datetime('%Q')` and `datetime_utc('%Q')` panic with exit code 101. `--timestamp --timestamp-format '%Q'` has the same problem.

12. **[P3] Cross-module aliases are omitted or attributed to another recipe.**  
    [subcommand.rs:552](/work/crate/src/subcommand.rs:552) groups aliases by the target’s bare recipe name. `alias shortcut := m::task` disappears from `--list` when no root `task` exists. If a root `task` exists, the listing incorrectly presents `shortcut` as its alias.

Validation: `cargo build --locked --bin just` passed. The original tests had 513 unit passes and 1,240 integration passes, with five environment-related failures. After supplying `USER` and `XDG_RUNTIME_DIR` and excluding three sandbox-sensitive permission/filename tests, all 513 unit and 1,242 integration tests passed; eight remained ignored.