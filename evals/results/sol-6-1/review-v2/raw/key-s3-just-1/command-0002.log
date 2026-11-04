Found **8 issues**, ordered by impact on ordinary use. Seven behavior findings were reproduced against the unmodified binary; the eighth is specific to the installed Clippy toolchain.

1. **Shared source files mix module identities and overrides.**  
   [src/compiler.rs:19](/work/crate/src/compiler.rs:19)

   The compiler caches parsed sources by filesystem path, although parsed sources contain module-specific paths and assignment identifiers. The documented module isolation requires each module instance to retain its own identity and values.

   Reproduced with `mod first 'shared.just'` and `mod second 'shared.just'`, where `task` prints `module_path()`: running `just first::task second::task` prints only `second` once. The second invocation is incorrectly treated as already completed.

   Separately, when distinct module files both import `common.just`, overriding `first::x` also changes `second::x`. Callers reusing common definitions can silently skip work or apply configuration to the wrong module. Both cases are covered by the scratch regressions.

2. **Formatting removes `[private]` from aliases and modules.**  
   [src/alias.rs:42](/work/crate/src/alias.rs:42), [src/item.rs:59](/work/crate/src/item.rs:59)

   Formatting should preserve the documented visibility behavior. Alias formatting omits its attributes, and module formatting ignores its `private` field.

   Executed `just --list`, `just --fmt`, then `just --list` on private aliases and modules. Formatting succeeds, removes `[private]`, and makes previously hidden entries appear.

   The existing round-trip helper only compares successive dumps ([tests/test.rs:369](/work/crate/tests/test.rs:369)); that verifies idempotence after information has already been lost.

3. **User-defined functions ignore variable overrides and reevaluate module assignments.**  
   [src/evaluator.rs:226](/work/crate/src/evaluator.rs:226), [src/evaluator.rs:241](/work/crate/src/evaluator.rs:241)

   Functions may reference assignments in their module, and command-line overrides should determine those assignments’ values. Function evaluation instead creates a fresh root scope and an empty override map.

   With:

   ```just
   set unstable
   x := 'original'
   f() := x
   show:
     @echo {{x}} {{f()}}
   ```

   `just --set x override show` prints `override original`, instead of `override override`. The same problem occurs through `--evaluate`. Callers receive inconsistent configuration depending on whether they access a variable directly or through a function. This affects the explicitly unstable function feature.

4. **Shell-format evaluation corrupts values containing `!`.**  
   [src/justfile.rs:323](/work/crate/src/justfile.rs:323)

   Shell-formatted assignments should recreate the evaluated values when loaded by a shell. For `greeting := 'Hello!'`, `just --evaluate --evaluate-format shell` emits:

   ```sh
   greeting="Hello\!"
   ```

   Executing that output with either `sh -c` or `bash -c` produces the value `Hello\!`, including the added backslash. Both shell round-trip regressions fail.

   The existing test at [tests/evaluate.rs:264](/work/crate/tests/evaluate.rs:264) asserts the incorrect emitted text without checking the resulting shell value.

5. **Relative or unnormalized ceilings fail to stop justfile searches.**  
   [src/search.rs:196](/work/crate/src/search.rs:196), [src/search.rs:246](/work/crate/src/search.rs:246)

   `--ceiling` promises to prevent searching above the specified directory. Its path is compared directly against absolute ancestor paths without resolving relative paths or cleaning `..`.

   From a child directory without a justfile, `just --ceiling .` finds and executes the parent’s recipe, exiting successfully. Passing the child’s absolute path correctly returns `error: No justfile found`. An equivalent absolute path containing `child/../child` also fails to constrain the search.

   Callers using ordinary relative paths can unexpectedly run recipes from another project. The absolute-path control passes; the relative-path regression fails.

6. **Invalid date formats panic instead of producing ordinary errors.**  
   [src/function.rs:245](/work/crate/src/function.rs:245), [src/function.rs:249](/work/crate/src/function.rs:249), [src/config.rs:137](/work/crate/src/config.rs:137)

   Invalid user-supplied formatting strings should produce useful errors. These paths call Chrono’s formatting implementation followed by `to_string()` without validating the format.

   Executed all three cases:

   - `datetime('%Q')`
   - `datetime_utc('%')`
   - `just --timestamp --timestamp-format '%Q'`

   Each exits **101** with `a Display implementation returned an error unexpectedly`, rather than an application diagnostic. The timestamp case prevents the recipe from running. Existing date and timestamp tests cover valid formats only.

7. **Cross-module aliases disappear from listings or attach to the wrong recipe.**  
   [src/subcommand.rs:555](/work/crate/src/subcommand.rs:555), [src/subcommand.rs:568](/work/crate/src/subcommand.rs:568)

   Public aliases should remain discoverable through `--list`. Listing groups aliases solely by the target recipe’s basename, then associates them with recipes in the current module.

   With `alias shortcut := helper::task`, `just shortcut` executes successfully, but both default listing and `--alias-style separate` omit `shortcut`. Adding an unrelated root recipe named `task` causes the listing to label that root recipe with `[alias: shortcut]`.

   Callers either cannot discover the alias or receive misleading information about its target. Both variants were executed.

8. **The configured Clippy check fails on Rust 1.99.1.**  
   [src/alias.rs:17](/work/crate/src/alias.rs:17), [src/invocation_parser.rs:333](/work/crate/src/invocation_parser.rs:333)

   `cargo clippy --locked --all --all-targets --all-features -- --deny warnings` fails on `clippy::manual_assert_eq`.

   Allowing that lint exposes `clippy::assert_is_empty` failures at `src/invocation_parser.rs:333`, `:404`, `:648`, and supporting test `tests/choose.rs:243`. Allowing both lints makes the full check pass.

   This blocks the project’s lint workflow on the supplied toolchain. It does **not** establish a failure on the declared Rust 1.85 minimum version.

Cargo checks ran from `/work/crate` on aarch64 Linux with Rust/Cargo **1.99.1**, `RUSTUP_AUTO_INSTALL=0`, and two build jobs.

| Command | Outcome |
|---|---|
| `cargo build --locked --bin just` | Passed |
| `cargo test --locked` | 513 unit tests passed; 1,240 integration tests passed, five failed, eight ignored |
| `TMPDIR=/target/review-test-tmp XDG_RUNTIME_DIR=/target/review-runtime USER=review cargo test --locked` | 513 unit and 1,245 integration tests passed; eight ignored |
| Same environment, `cargo test --locked --workspace` | Passed; eight ignored |
| `cargo clippy --locked --all --all-targets --all-features -- --deny warnings` | Failed as described above |
| Same Clippy command plus `-A clippy::manual_assert_eq` | Failed on four `assert_is_empty` occurrences |
| Same Clippy command plus both `-A clippy::manual_assert_eq -A clippy::assert_is_empty` | Passed |
| `cargo fmt --all -- --check` | Passed |
| `RUSTDOCFLAGS='-D warnings' cargo doc --locked --no-deps --all-features` | Passed |
| `python3 /work/tmp/review/regressions.py` | 14 tests executed: 12 failures covering the seven behavior findings; two controls passed |

The five initial integration failures were environmental: missing `USER` and `XDG_RUNTIME_DIR`, `/work` permitting writes to read-only files, and two non-Unicode directory-creation failures on `/work`. Local filesystem probes confirmed those differences; relocating temporary files to `/target` resolved them without source changes.

No production files were edited. Added fixtures, harnesses, and logs are in `/work/tmp`, with filesystem-check directories under `/target`. The [regression harness](/work/tmp/review/regressions.py) and [results](/work/tmp/review/regressions.log) preserve the reproductions.

**Finding count: 8.** Coverage was Linux-only; the eight ignored tests, Rust 1.85, Miri, and other platforms were not tested. Rustdoc passed; doctests are disabled by the crate. The mdBook check was unrun because `mdbook` is unavailable. No dedicated security testing was performed. Last clock reading: **7.4498 of 45 minutes used**.