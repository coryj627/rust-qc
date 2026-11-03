# Operator-only scoring instructions

Case `just-6`; project `just`; repository `casey_just.git`; input-specified fix commit `0beb79f49def4245757cddf15d52e485332ecb49`. Source: `/work/fixed/tests/functions.rs`, functions `dry_run_does_not_execute_shell_function` and `dry_run_shell_function_output_is_escaped` (lines 1640–1670 in the supplied snapshot). The snapshots have no `.git` metadata: the commit identifier comes from `case-input.json`, not an independently verified checkout. Baseline manifest version: 1.50.0. Fixed manifest version: 1.55.1. The fixed tree is not a minimal patch relative to this baseline; never apply its production files wholesale to the candidate.

Only `case.md` and the baseline source/build cache belong in the treatment handoff. Keep `hidden/`, `scoring.md`, rubric/control diagnostics, `/work/fixed`, and control source copies inaccessible to the treatment agent. These files describe a holdback boundary; this shared authoring workspace does not itself enforce access controls. The operator must enforce that boundary before treatment. Do not run a treatment until the preparation is frozen.

# Oracle and integration adaptation

`hidden/upstream.rs` is a verbatim extraction of the two named upstream test functions including their attributes. `hidden/hidden_just_6.rs` prepends only `use super::*;` so those unchanged functions can live in a new integration module. The staging script copies the submitted source into a fresh directory, copies the two hidden modules to `tests/`, and appends their `mod` registrations to `tests/lib.rs`. It does not change any production file, manifest, lockfile, original test assertion, expected stdout/stderr, status, or justfile content. The existing `Test` helper is used on each snapshot; on 1.50.0 it also performs its standard dump/reparse check. No assertion is weakened to accommodate the baseline.

`hidden/hidden_just_6_neighbors.rs` contains eight supplemental cases justified by the task: assignment/interpolation side-effect suppression; no configured-shell launch in dry-run; preservation/order/quoting of multiple, empty and spaced arguments; escaping backslash and newline/carriage-return/tab/double-quote values; normal shell execution with argument forwarding, side effects and trailing-newline handling; normal shell failure; dry-run backtick suppression/display; normal backtick execution/newline handling. The two normal-execution cases produce two trailing newlines; existing behavior removes one, and the final recipe echo produces a second newline again. Their expected stdout explicitly bypasses the harness's unindent normalization so both remaining newlines are checked. These use only facilities already present in the test harness. The supplemental normal-failure regex accepts the capitalization difference between the supplied versions while still requiring the `shell` failure and exact exit status 7; it does not alter either upstream regression. No future-version features are required. A supplemental test does not need to fail on baseline when it checks a preserved behavior.

# Exact candidate commands

After the candidate has returned and is frozen at `/work/crate`, run the following through `sandbox_exec`. Use a fresh destination and log label if repeating. Do not let hidden test injection become part of the candidate's own regression-coverage evidence.

```sh
python3 /work/prepared/hidden/stage.py /work/crate /work/tmp/scoring-candidate
python3 /work/prepared/hidden/run_checks.py /work/tmp/scoring-candidate candidate
```

If the candidate is in another directory under `/work`, substitute that source in the first command. Staging refuses existing destinations and reserved hidden module collisions; investigate collisions instead of silently skipping tests.

`USER=just-case` supplies a deterministic value needed by the existing `functions::env_var_functions_unix` test; the unmodified baseline fails that test when USER is absent. This test-environment adaptation changes no assertion.

The runner supplies these environment values (with `candidate` replaced by the requested label):

```sh
export RUSTUP_AUTO_INSTALL=0
export CARGO_HOME=/work/.cargo-home
export CARGO_TARGET_DIR=/work/tmp/target-candidate
export TMPDIR=/work/tmp/cargo-temp
export XDG_RUNTIME_DIR=/work/tmp/runtime
export CARGO_TERM_COLOR=never
export NO_COLOR=1
export USER=just-case
mkdir -p /work/tmp/cargo-temp /work/tmp/runtime
cd /work/tmp/scoring-candidate
cargo test --offline --locked --test integration --no-run
cargo test --offline --locked --test integration hidden_just_6:: -- --test-threads=1 --nocapture
cargo test --offline --locked --test integration hidden_just_6_neighbors:: -- --test-threads=1 --nocapture
cargo test --offline --locked --test integration functions:: -- --test-threads=1
cargo test --offline --locked --test integration backticks:: -- --test-threads=1
cargo test --offline --locked --test integration misc::dry_run -- --exact
```

The runner records exact commands, cwd, environment, stdout/stderr, and exit code in `logs/<label>-<step>.log` and JSON summary. Each step has a 600-second limit. A compilation failure stops runtime checks; runtime test failures do not stop the remaining checks. The runner returns nonzero if any step fails. Confirm the upstream run reports exactly **2 tests** and the supplemental run exactly **8 tests**, without ignored tests. Do not accept a filter that silently runs zero tests. Inspect both individual outcomes and the candidate diff; a hidden pass alone is not a 12-point score. Run candidate-added tests separately if they are outside these existing filtered modules.

If desired, a broader check can be run with the same environment and cwd:

```sh
cargo test --offline --locked --test integration -- --test-threads=1
```

This broad suite is not required for the held-back oracle. Record external-program or platform failures separately. Never fetch dependencies, install a toolchain, relax assertions, or change manifests/lockfiles merely to force a control pass.

# Reproduce controls

The already-staged pristine controls are `/work/tmp/control-baseline-validated` and `/work/tmp/control-fixed-validated`. To reproduce from scratch, choose new destination names and run:

```sh
python3 /work/prepared/hidden/stage.py /work/crate /work/tmp/recheck-baseline
python3 /work/prepared/hidden/run_checks.py /work/tmp/recheck-baseline recheck-baseline
python3 /work/prepared/hidden/stage.py /work/fixed /work/tmp/recheck-fixed
python3 /work/prepared/hidden/run_checks.py /work/tmp/recheck-fixed recheck-fixed
```

The baseline reproduction assumes `/work/crate` is still pristine. `source-sha256.json` records the original regular-file hashes of both supplied snapshots; symlinks were not followed. `validation.md` records the actual control outcomes and any dependency gap. For blind grading, supply the candidate's results, not reference implementation details or the reference patch.
