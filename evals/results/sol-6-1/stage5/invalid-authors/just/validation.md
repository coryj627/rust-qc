# Validation report

Preparation artifacts are written. Baseline regression detection is verified. **Fixed-control runtime validation is incomplete because its dependencies are unavailable offline; no fixed pass is claimed.** No candidate fix or treatment run was performed. Neither original production tree was modified.

| Check | Actual result | Evidence |
|---|---|---|
| Pristine baseline copy, integration compilation | Pass, exit 0 | `logs/baseline-compile.log` |
| Unchanged upstream regression functions on baseline | Both fail, exit 101; each executes `exit 1` through `shell()` during dry-run, returning status 1 instead of 0 and the command failure instead of the expected unevaluated call | `logs/baseline-upstream.log` |
| Supplemental cases on baseline | 4 fail for intended dry-run behavior; 4 compatibility cases pass; exit 101 | `logs/baseline-neighbors.log` |
| Existing baseline function tests | 88 pass, exit 0 | `logs/baseline-functions.log` |
| Existing baseline backtick tests | 2 reported passing, exit 0; the PowerShell-only case returns early on Linux, so this is one exercised Linux case plus one platform skip encoded as success | `logs/baseline-backticks.log` |
| Existing baseline dry-run test | 1 pass, exit 0 | `logs/baseline-existing-dry-run.log` |
| Pristine fixed copy, integration compilation | Blocked, exit 101: `no matching package named digest-io found`, required by just 1.55.1; searched the offline crates.io index | `logs/fixed-compile.log` |
| Fixed-control runtime tests | Not executed because compilation failed; neither upstream nor supplemental tests have a verified fixed-control pass | `logs/fixed-summary.json` |
| Source integrity and test extraction | Both source trees retain all original regular-file hashes; control copies differ only by the two hidden test modules and module registrations; original test bytes/assertions match exactly | `logs/integrity.log`, `source-sha256.json` |

The four supplemental baseline failures concern side-effecting assignment/interpolation calls, escaping control characters, a missing configured shell executable, and empty/spaced/multiple argument display. The four passes preserve normal shell output/arguments/side effects, normal shell errors, dry-run backticks, and normal backtick execution/newline handling. These are expected baseline outcomes, not four unrelated failures. The upstream regressions alone provide the decisive baseline failure evidence.

The first fixed-control blocker is missing `digest-io` (locked version 0.1.0). A static comparison of the fixed lockfile with available registry archives and extracted sources found 53 absent package/version entries, listed in `logs/fixed-cache-gaps.txt`; some may be platform-specific and that inventory does not claim all would be built on Linux. Other direct-dependency gaps include `clap_complete` 4.6.5, `sha2` 0.11.0, and `pulldown-cmark` 0.13.4. No dependency was fetched, toolchain installed, manifest relaxed, or reference source changed. The fixed snapshot reports version 1.55.1, whereas baseline reports 1.50.0, explaining why a baseline-populated cache need not build the fixed control.

The commit provenance is supplied by `case-input.json`: `just-6`, repository `casey_just.git`, revision `0beb79f49def4245757cddf15d52e485332ecb49`, test source `tests/functions.rs:1640–1670`. Neither snapshot includes `.git`, so commit identity could not be independently verified. Test extraction hashes and adaptation details are in `hidden/provenance.json`.

# Preparation corrections and environment

Initial exploratory control logs are retained in `logs/initial/`. That first run exposed two authoring errors in supplemental normal-execution expectations: existing shell/backtick behavior strips one trailing newline, not every trailing newline; and the test harness normalizes expected stdout by default. The final cases assert the existing two-newline observable output explicitly with stdout normalization disabled. These corrections changed only supplemental tests, never either upstream regression. Fresh source copies were staged and all reported final baseline checks were rerun.

The initial existing function suite had one unrelated failure because this container lacks `USER`. The final runner supplies `USER=just-case`; all 88 pre-existing function tests then pass without changing their assertions. The runner also directs test/build temporary files and runtime directories under `/work/tmp`, uses the installed default Cargo toolchain with `RUSTUP_AUTO_INSTALL=0`, and runs Cargo with `--offline --locked`. The installed toolchain version is recorded in `logs/integrity.log`.

# Handoff limitations

This authoring workspace has shared filesystem access. Merely placing files in `hidden/` cannot prevent a treatment agent from opening them. The treatment operator must expose only the case and baseline/build cache, withhold the reference source and operator material, and enforce this boundary before treatment. The rubric is fixed for two independent blind graders; no treatment output has informed it.

A dependency-complete execution of the pristine fixed control remains necessary to establish the requested positive-control pass. The exact commands and staging procedure are in `scoring.md`. In this offline workspace, report this limitation instead of substituting a backported production patch or claiming validation from source inspection. The Linux runs do not validate Windows-specific shell execution. The full integration suite was not run; the targeted existing compatibility suites were run successfully.
