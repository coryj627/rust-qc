# Preparation validation report

Prepared from `/work/case-input.json` before any treatment run. No candidate fix was implemented. Baseline production files were not changed. Source manifests and integrity results cover both original snapshots and all original files in the copied controls.

## Controls

| Check | Baseline | Supplied fixed source |
| --- | --- | --- |
| Native Cargo oracle, debug | Intended failure: subtraction overflow, parse.rs:892; exit 101 | Blocked by offline `dlmalloc` resolution; test did not execute |
| Native Cargo oracle, release | Intended failure: inverted byte slice, parse.rs:911; exit 101 | Same dependency block; test did not execute |
| Native hidden neighbors, debug/release | Malformed test fails; three compatibility tests pass in each profile | Native run unavailable for the same workspace dependency gap |
| Supplemental direct-source oracle, debug | Native baseline oracle establishes failure | 1 passed, 0 failed |
| Supplemental direct-source oracle, optimized / overflow checks off | Native baseline release oracle establishes failure | 1 passed, 0 failed |
| Supplemental direct-source neighbors, debug and optimized | Native baseline neighbors establish malformed failure and compatibility | 4 passed, 0 failed in each configuration |
| Existing unit/integration suites, debug and release | 1,250 passed in each profile (59 + 23 + 20 + 1,148); gated serde target ran 0 | Not run natively; no claim of full fixed-suite coverage |
| Doctests, debug and release | 5 passed in each profile | Not run |
| No default features, Cargo check | Pass | Not run; fixed feature model also differs from baseline |

The supplemental checks compile pristine fixed parser and escaping production source without edits, with the fixed default library feature cfgs. Only the upstream test's integration placement/imports are adapted; its original assertion and body are preserved exactly. Neighboring tests are separate and follow the task's safety and preservation requirements. Both hidden fixtures pass the unchanged fixed production source under the direct compilation method.

## Dependency and build limitations

The supplied fixed workspace adds `guide/demo`, whose `pulldown-cmark-demo v0.0.0` package requires `dlmalloc`. The baseline offline cache cannot resolve that package, so native Cargo debug/release fixed controls stop before compiling the parser. This is the first observed dependency gap; later fixed dependencies were not exhaustively checked. No tools or dependencies were installed, and no network was used.

To obtain a supplemental fixed-source control without altering production or manifest files, the installed compiler used already-built baseline debug/unwind dependency rlibs (`bitflags`, `memchr`, `unicase`, `getopts`); the fixed escaping crate was compiled from its own source. Parser/escaping builds used either `opt-level=0, overflow-checks=yes` or `opt-level=3, overflow-checks=no`. The latter verifies optimized parser arithmetic and rendering, but does not constitute a complete native Cargo release build: cached dependencies remain debug/unwind, Cargo workspace resolution and release LTO are bypassed, and other fixed feature combinations are unverified. Initial use of release dependency artifacts encountered an abort/unwind mismatch; final reproducible commands use the unwind artifacts for both configurations. Three pre-existing deprecated `u8` warnings were emitted by fixed source.

The minimum declared Rust version was not tested; only installed rustc/cargo 1.99.1 were used. Feature-gated serde/simd/gen-tests combinations were not part of the controls. `gen-tests` was not enabled, so no generated source suites were rewritten.

## Artifact checks and replay

- `case.md` includes the original task text verbatim, with build/workspace instructions and no hidden inputs or implementation hints.
- `hidden/upstream-original.rs` matches the fixed source's named attributed function byte-for-byte. The adapted fixture contains that function unchanged after its import prelude.
- `logs/source-integrity.json` confirms every original file is unchanged in both snapshots and both controls; control copies add only the two hidden integration files.
- The candidate replay helper was syntax checked and executed against baseline in separate self-check locations. It returned 1 as expected: debug/release oracle and neighbor runs fail intended behavior, full suites fail on injected malformed cases, and no-default-features check passes. No candidate or treatment output was used.
- Command logs omit routine successful per-test lines while retaining summaries, commands, warnings, and failure diagnostics. Compiler/test commands for the supplemental controls are retained verbatim in `logs/fixed-direct-commands.json`.

See `scoring.md` for exact post-return injection/replay commands and the limitations to report to graders. The runner must expose only `case.md` and the baseline repository to the treatment agent; hidden-directory naming alone does not enforce withholding.
