# Private scoring procedure — just-6

Bug ID: `just-6`; source repository: `casey_just.git`; reference commit supplied in case-input.json: `0beb79f49def4245757cddf15d52e485332ecb49`. Original oracle source: `/work/fixed/tests/functions.rs`, functions `dry_run_does_not_execute_shell_function` and `dry_run_shell_function_output_is_escaped`. See hidden/provenance.json for hashes and extraction details. No candidate fix was authored during preparation.

## Visibility and source preservation

Before a treatment starts, provide only case.md and an isolated copy of the baseline source as its workspace. Do not expose the fixed source, private grading materials, control copies, authoring scratch files, build outputs, or logs. Enforce this in the treatment's filesystem/mount configuration; this preparation directory alone cannot enforce access separation in the shared authoring container. The baseline's existing public tests remain available. Do not distribute this document to treatment agents.

The supplied `/work/crate` and `/work/fixed` trees are read-only references for scoring. Apply held-back tests only to copies. No production patch, manifest, lockfile, dependency, or toolchain modification is part of integration adaptation. Tests are registered in `tests/lib.rs` under two new module names. The original function names, test attributes, bodies, and assertions are preserved exactly; adding a module prefix is the only test-name adaptation. The fixed tree already contains its own original tests; they are retained, not overwritten. Nine independent neighboring tests are justified in hidden/README.md.

## Exact candidate commands

Place the complete returned candidate source at `/work/candidate` (or replace that single source argument below with its actual path). Use a fresh `/work/tmp/candidate-score` destination; do not reuse a stale scoring source directory. The following copies candidate source, preserving symlinks and excluding Git/build output, and installs the two held-back modules:

```sh
python3 - /work/candidate /work/tmp/candidate-score <<'PY'
from pathlib import Path
import shutil
import sys
source, destination = map(Path, sys.argv[1:3])
shutil.copytree(source, destination, symlinks=True,
                ignore=shutil.ignore_patterns('.git', 'target'))
PY
python3 /work/prepared/hidden/apply_hidden.py /work/tmp/candidate-score
python3 /work/prepared/hidden/run_checks.py /work/tmp/candidate-score /work/prepared/logs/candidate
```

The runner uses the installed toolchain and the existing cache, with `CARGO_HOME=/work/.cargo-home`, `CARGO_TARGET_DIR=/work/tmp/just-6-control-target`, `RUSTUP_AUTO_INSTALL=0`, `TMPDIR=/work/tmp`, `CARGO_NET_OFFLINE=true`, and `CARGO_BUILD_JOBS=4`. It executes these exact commands from the scoring source copy, in order:

```sh
cargo test --offline --locked -p just --test integration --no-run
cargo test --offline --locked -p just --test integration case_just_6_upstream:: -- --nocapture
cargo test --offline --locked -p just --test integration case_just_6_neighbors:: -- --nocapture
cargo test --offline --locked -p just --test integration functions::shell_ -- --nocapture
cargo test --offline --locked -p just --test integration misc::dry_run -- --exact --nocapture
cargo test --offline --locked -p just --test integration backticks:: -- --nocapture
```

Runner exit 0 means these checks passed; exit 1 means at least one behavior check failed; exit 2 means compilation/dependency resolution failed and no behavior tests were run. Each command's exit code and full output are stored separately in the specified logs directory, with a machine-readable results.json. Require **2 upstream tests and 9 neighboring tests actually executed**, with no ignored tests; a zero-test success is not evidence of correctness. Existing filters should execute six shell tests, one dry-run test, and two backtick tests in these supplied snapshots. Inspect test names and platform guards: the PowerShell checks return early on Linux, and their nominal pass does not validate Windows.

Review candidate-authored regression tests separately for rubric R; held-back modules do not count as candidate work. To run the candidate's entire registered integration suite, use the following from `/work/tmp/candidate-score` with the same environment (this broader suite was not run as an authoring control):

```sh
CARGO_HOME=/work/.cargo-home CARGO_TARGET_DIR=/work/tmp/just-6-control-target RUSTUP_AUTO_INSTALL=0 TMPDIR=/work/tmp CARGO_NET_OFFLINE=true CARGO_BUILD_JOBS=4 cargo test --offline --locked -p just --test integration
```

Use the 12-point rubric and separate harm assessment independently for both blind graders. Do not make a binary test pass/fail into a substitute for that rubric. Keep failed logs. Treat compile/cache/tooling failures separately from observable behavior failures. Do not fetch/install anything or adjust manifests to force a control to pass. Candidate-introduced dependency failures should be attributed to that change; genuine external cache gaps should be documented precisely.

## Reproducing the authoring controls

The pristine source copies were created under `/work/tmp/baseline-control` and `/work/tmp/fixed-control`, preserving symlinks. Their ordinary files were byte-checked against the original snapshots before integration. Only the two new test modules and their registrations were subsequently added. From those copies, the already executed control commands were:

```sh
python3 /work/prepared/hidden/apply_hidden.py /work/tmp/baseline-control
python3 /work/prepared/hidden/apply_hidden.py /work/tmp/fixed-control
python3 /work/prepared/hidden/run_checks.py /work/tmp/baseline-control /work/prepared/logs/baseline
python3 /work/prepared/hidden/run_checks.py /work/tmp/fixed-control /work/prepared/logs/fixed
```

A rerun of the baseline runner is expected to exit 1; that is the intended negative control. The fixed runner must exit 0. No compatibility shim or production adaptation is allowed to change this distinction.

## Observed control results and limitations

| Check | Baseline 1.50.0 | Fixed snapshot 1.55.1 |
|---|---|---|
| Compile integration target offline/locked | Pass | Pass |
| Two verbatim upstream regressions | 0 pass, 2 fail | 2 pass |
| Nine specification-derived neighbors | 3 pass, 6 fail | 9 pass |
| Existing shell filter | 6 nominal pass | 6 nominal pass |
| Existing dry-run test | 1 pass | 1 pass |
| Existing backtick filter | 2 nominal pass | 2 nominal pass |

Baseline upstream failures are caused by real execution of `shell('exit 1')`: status 1 and a shell-call failure diagnostic replace the expected status 0 and unevaluated call output. Neighbor failures are confined to dry-run shell behavior; the three neighboring compatibility checks pass on baseline. In the configured-shell neighbor the harness override is disabled, so baseline fails while trying to launch the deliberately nonexistent interpreter. The fixed control passes all selected checks without source/dependency alterations. Both snapshots' dependencies were available offline; no dependency gap remains.

Validation used installed cargo 1.99.1 and rustc 1.99.1 on Linux with Bash. No minimum-Rust-version, Windows, or full-suite validation was performed. The fixed snapshot contains unrelated evolution beyond the baseline; its package version is 1.55.1 and its identity as the supplied commit is taken from case-input.json, since neither snapshot has Git history. Do not require candidates to reproduce unrelated version, dependency, or implementation changes from that snapshot. The fixed manifest emits a pre-existing deprecated lint-name warning, which does not affect the selected checks.

See logs/validation-summary.md for concise results, logs/{baseline,fixed}/ for actual command output/exit codes, logs/source-manifests.json for initial source hashes, and logs/integrity.json for the final unchanged-source and upstream-extraction audit.
