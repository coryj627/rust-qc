Found **7 correctness issues**, all reproduced locally. The supplied source was unchanged; scratch tests are under `/work/tmp/review`.

1. **High — `shell()` executes during `--dry-run`.**  
   [src/function.rs:548](/work/crate/src/function.rs:548) calls the command evaluator without checking `dry_run`.

   With `x := shell('echo called >> marker; echo hello')` interpolated into a recipe, `just --dry-run foo` creates `marker` and exits 0. Expected: print the proposed recipe without executing its shell expressions. This makes previewing recipes perform work. The equivalent backtick expression correctly leaves the marker absent.

2. **Medium — Loading one file as two modules conflates their identities.**  
   [src/compiler.rs:19](/work/crate/src/compiler.rs:19) caches parsed sources solely by filesystem path, although the AST contains module-specific context.

   For:
   ```just
   mod a 'common.just'
   mod b 'common.just'
   ```
   with `common.just` containing a recipe that prints `module_path()`, `just a::foo b::foo` prints only `b` once. Expected: execute both distinct module recipes and print `a`, then `b`. Invoking `a::foo` alone also prints `b`. Separate files containing identical text correctly print both names. Ordinary callers can silently skip requested recipes or receive the wrong module context.

3. **Medium — Dependency argument grouping can panic during compilation.**  
   [src/recipe.rs:101](/work/crate/src/recipe.rs:101); argument-count validation occurs at [src/unresolved_recipe.rs:78](/work/crate/src/unresolved_recipe.rs:78).

   Reproducer:
   ```just
   [arg('opt', long)]
   foo arg='default' opt:
     @echo {{arg}} {{opt}}

   bar: (foo 'option-value')
   ```

   `just bar`, `just --list`, and even `just foo --opt option-value` panic with `assertion failed: parameter.default.is_some()` and exit 101. Expected: bind arguments consistently or report an invalid dependency through the normal diagnostic path. The signature itself works when the dependency declaration is removed. One declaration therefore prevents unrelated recipes and listing from working.

4. **Medium — User-defined functions lose assignment overrides and cached values.**  
   [src/evaluator.rs:226](/work/crate/src/evaluator.rs:226) creates a fresh root scope, and line 241 creates an empty override map.

   With:
   ```just
   x := 'original'
   get() := x
   a := get()
   ```
   `just --unstable --evaluate --set x override` prints `a := "original"` alongside `x := "override"`. Expected: the function sees the overridden assignment. A second test using a backtick assignment recorded two evaluations when accessed directly and through a function. Callers get inconsistent values and repeated computation. This affects the documented, currently unstable function feature.

5. **Medium — Invalid datetime formats panic instead of returning errors.**  
   [src/function.rs:246](/work/crate/src/function.rs:246), [src/function.rs:250](/work/crate/src/function.rs:250), and [src/config.rs:137](/work/crate/src/config.rs:137).

   Evaluating `datetime('%Q')`, evaluating `datetime_utc('%')`, and running with `--timestamp --timestamp-format '%Q'` each exit 101 with `a Display implementation returned an error unexpectedly`. Expected: a normal invalid-format diagnostic. These functions call `.to_string()` on Chrono formatting results that can return formatting errors. A format typo interrupts normal CLI execution and can unwind through the public library entry point.

6. **Medium — Relative search ceilings are ignored.**  
   [src/search.rs:196](/work/crate/src/search.rs:196) compares absolute search directories with an unnormalized ceiling.

   From an empty child directory whose parent contains a justfile, `just --ceiling .` executes the parent recipe and exits 0. Supplying the child’s absolute path correctly returns `No justfile found`, exit 1. Expected: both paths impose the same ceiling. Callers using ordinary relative paths can select a justfile above their requested search boundary.

7. **Medium — `--quiet` does not suppress script output.**  
   [src/recipe.rs:541](/work/crate/src/recipe.rs:541) launches script recipes without the output redirection used for linewise recipes.

   A shebang recipe containing `echo out` and `echo err >&2` prints both streams under `just --quiet foo`, exiting 0. `[script('sh')]` behaves identically; the equivalent linewise recipe produces no output. Expected: the documented “Suppress all output” behavior applies consistently. Scripts used in quiet automation therefore produce unexpected output.

Executed checks, using Rust 1.99.1 on aarch64 Linux with `RUSTUP_AUTO_INSTALL=0`:

- From `/work/crate`, `cargo build --locked --bin just`: **passed**.
- `cargo test --locked`: **513 unit tests passed**; integration tests had **1,240 passed, 5 failed, 8 ignored**.
- Rerunning with `USER=audit XDG_RUNTIME_DIR=/work/tmp/review/runtime`: **513 unit tests passed; 1,242 integration tests passed, 3 failed, 8 ignored**. Remaining failures were `format::write_error`—writing succeeded despite its read-only setup—and two non-Unicode tests whose directory creation failed with OS error 84.
- `cargo fmt --all -- --check`: **passed**.
- `cargo clippy --locked --all-targets`: **failed** on four denied assertion-style lint diagnostics: one `manual_assert_eq` and three `assert_is_empty`.
- A separate Rust caller successfully exercised repeated `just::run()` calls, `summary()`, and `unindent()`.

The finding reproducers and controls can be rerun with:

```sh
python3 /work/tmp/review/probes.py
python3 /work/tmp/review/more_probes.py
python3 /work/tmp/review/controls.py
```

Source inspection focused on parsing, dependency resolution, evaluation, module loading, command execution, error handling, and public entry points, with documentation and tests as supporting material. This was not exhaustive branch coverage. Release builds, other operating systems, MSRV compatibility, Miri, and the eight ignored tests were not run. Security testing, exploitability analysis, and inquiries requiring dedicated security testing remain outside scope.

**Finding count: 7. Last `/work/clock` reading: 4.7236 minutes used of 15.0.**