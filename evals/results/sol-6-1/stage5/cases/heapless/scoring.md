# Evaluator-only preparation and scoring

Do not distribute this file, `hidden/`, `rubric.md`, validation logs, `/work/fixed`, or control workspaces to the treatment agent. Supply only `case.md`, the baseline source, the existing offline cache/toolchain, and an empty temporary workspace. Hiding a path in prose is not access isolation: the treatment runner must withhold these materials. No treatment output was consulted and no candidate fix was implemented during preparation.

## Provenance and oracle integrity

- Project/repository: heapless / `rust-embedded_heapless.git`.
- Case ID: `heapless`; bug ID: `heapless-6`.
- Exact upstream fixed commit, supplied by case-input.json: `906f8ae525fc8f019886f5fb39d9878557611de5`.
- Source: `/work/fixed/src/index_map.rs`, lines 2114–2128; named oracle: `src/index_map.rs::tests::truncate_invalid` (Rust test path `index_map::tests::truncate_invalid`). The source comment links upstream issue https://github.com/rust-embedded/heapless/issues/674. The case bug ID and upstream issue number are distinct identifiers.
- `hidden/upstream-truncate-invalid.rs` is the verbatim upstream doc comment, test attribute, and function, including every original assertion and comment. `hidden/heldback_truncate_upstream.rs` prepends only `use heapless::index_map::FnvIndexMap;` so that the unchanged function runs as an integration test. `Vec`/`vec!` retain their standard test-prelude meaning. Test inputs, assertions, order, and types are unchanged. This adaptation avoids editing candidate production files or depending on their unit-test module layout. The integration test name is `truncate_invalid`.
- `hidden/heldback_truncate_spec.rs` adds only task-derived neighboring checks: all 24 insertion orders of four keys, all five populated-prefix lengths, and eight requested lengths (960 starting-map/length combinations); retained lookup/membership/values/order/length and removed-key absence; repeated equal truncation; a custom constant hasher with a wrapping collision chain, non-Clone keys/values, and repeated truncation. Its two named tests are `fnv_truncation_matrix` and `custom_hasher_nonclone_entries_and_repeated_truncation`. It requires no added dependency. These cases enforce public behavior, not a particular repair strategy. No new panic/drop-unwind, timing, or unrelated collection contract is imposed.
- Original source file hashes are in `logs/crate-source-sha256.json` and `logs/fixed-source-sha256.json`. Neither original tree was modified. Control copies change only by adding integration test files and Cargo-generated artifacts; source integrity is recorded in `logs/integrity.log`.

## Exact candidate scoring command

After the candidate has returned its edits in `/work/crate`, run from the sandbox:

```sh
python3 /work/prepared/hidden/score_candidate.py /work/crate /work/tmp/scoring-candidate
```

Use a new run directory if the example path already exists. This script copies the returned candidate to `/work/tmp/scoring-candidate/source`, preserving its manifest and lockfile. It runs the candidate's own/default suite and library check first, then copies the two held-back integration test files into that disposable copy. It refuses to overwrite an existing same-named file. It never edits the submitted candidate or substitutes dependencies. All executable/build artifacts go under `/work/tmp`. It records exact commands, environment, output, and exit statuses under the run directory's `logs/`, plus `results.json`. It attempts every check despite earlier test failures; its final exit status is 0 only if all checks succeed. A hidden test-byte-integrity check runs afterward.

The four underlying commands are exactly:

```sh
cd /work/tmp/scoring-candidate/source
export CARGO_HOME=/work/.cargo-home
export CARGO_TARGET_DIR=/work/tmp/scoring-candidate/target
export CARGO_NET_OFFLINE=true
export RUSTUP_AUTO_INSTALL=0
export CARGO_TERM_COLOR=never
cargo test --offline --quiet
cargo check --offline --lib --no-default-features
# The scoring script now copies hidden/heldback_truncate_{upstream,spec}.rs into tests/.
cargo test --offline --test heldback_truncate_upstream -- --exact truncate_invalid --nocapture
cargo test --offline --test heldback_truncate_spec -- --nocapture
```

Verify that the oracle command actually runs 1 test, and the neighbors command runs 2 tests, with none ignored. Compilation/resolution errors are distinct from behavioral assertion failures. Review candidate changes for feature/test gating or other suppression before accepting a pass. Do not use the control-only manifest harness to bypass a candidate's manifest or dependency changes.

Give two blind graders the materials specified by `rubric.md`. Apply its 12 individual points and separate harm dimensions. Candidate-added tests must supply their own regression coverage: evaluator tests do not earn R1/R2. For R1, inspect or transfer only candidate test hunks to another pristine baseline copy, preserving the baseline production implementation, and run the candidate's named test. A compile failure from an invalid test transfer is not evidence of a behavioral regression test. Keep any such candidate-specific command and result with the score sheet. No candidate has returned yet, so no candidate-specific commands/scores exist in this preparation.

## Executed control validation

All commands used the installed Cargo 1.99.1 / rustc 1.99.1, the existing `/work/.cargo-home`, offline mode, and `RUSTUP_AUTO_INSTALL=0`. No tools, toolchains, or dependencies were installed. Source trees were copied with:

```sh
mkdir -p /work/tmp/control-baseline /work/tmp/control-fixed
cp -a /work/crate/. /work/tmp/control-baseline/
cp -a /work/fixed/. /work/tmp/control-fixed/
cp /work/prepared/hidden/heldback_truncate_upstream.rs /work/tmp/control-baseline/tests/
cp /work/prepared/hidden/heldback_truncate_upstream.rs /work/tmp/control-fixed/tests/
cp /work/prepared/hidden/heldback_truncate_spec.rs /work/tmp/control-baseline/tests/
```

The log runner invocation is `python3 hidden/run_logged.py LOG CWD TARGET_DIR COMMAND ARG...`; it records the environment and real command status. Every final result below has a log in `logs/`.

| Check | Outcome | Log |
| --- | --- | --- |
| Baseline, original manifest, preserved oracle | **Intended assertion failure**, exit 101: retained key lookup is `None` instead of `Some(222)` after truncation; preceding iteration assertion succeeds. | `native-baseline-upstream.log` |
| Baseline, original manifest, neighbors | **Intended assertion failure**, exit 101: matrix retained lookup fails; non-Clone/custom-hasher case passes. | `native-baseline-spec.log` |
| Baseline existing library unit tests | **PASS**, 186 tests. | `native-baseline-existing.log` |
| Baseline no-default-features library check | **PASS**. | `native-baseline-no-std.log` |
| Fixed, original manifest, preserved oracle | **BLOCKED before compilation**, exit 101: offline registry has no matching package `embedded-io`, required by fixed heapless 0.9.3. This is not a native test pass or failure. | `native-fixed-upstream.log` |
| Baseline unchanged source, external default-feature manifest, preserved oracle | **Same intended assertion failure**, exit 101. | `source-control-baseline-upstream.log` |
| Baseline unchanged source, external default-feature manifest, neighbors | **Same intended matrix failure**, exit 101; custom-hasher case passes. | `source-control-baseline-spec.log` |
| Fixed unchanged source, external default-feature manifest, all hidden tests | **PASS**, 3/3 tests. | `source-control-fixed-hidden.log` |
| Fixed unchanged source, external default-feature manifest, existing IndexMap tests | **PASS**, 25 tests, including the original in-module oracle. | `source-control-fixed-existing.log` |
| Fixed unchanged source, external default-feature manifest, no-default-features check | **PASS**. | `source-control-fixed-no-std.log` |

### Fixed native build gap and source-control adaptation

The supplied fixed tree identifies itself as heapless 0.9.3, whereas baseline is 0.9.1. It contains unrelated intervening changes and a newer manifest. Its original Cargo invocation cannot resolve the optional dependency `embedded-io = "0.7"` offline even with default features; Cargo resolves optional dependencies while producing a lockfile. The fixed manifest also declares optional `zeroize = "1.8"`, whose source is absent from the provided cache; after the first resolution error further native gaps were not enumerated. Do not claim the exact fixed native manifest passed. No network retry, fetch, or installation was attempted.

To still validate the exact fixed **production source**, `hidden/make_source_control.py` creates an external Cargo package with `[lib]` and build-script paths pointing into a pristine control source copy. It uses the same package version, Rust 2021, original build.rs, and only the actual default-feature runtime dependencies (`hash32` and `stable_deref_trait`), plus existing test dependencies (`critical-section`, `static_assertions`). Original feature names are declared as empty, disabled entries solely to recognize source cfg names. It enables no optional feature. No production file is patched, no upstream fix is transplanted, and the same harness is used for both controls. Cargo's minimal harness lockfile is separate from either source tree. This validates default-feature source behavior; it does **not** validate full native manifests, optional features, cross-target builds, or source compatibility of all unrelated intervening changes.

Exact harness creation commands (output directories must not exist):

```sh
python3 /work/prepared/hidden/make_source_control.py /work/tmp/control-baseline /work/tmp/source-control-baseline
python3 /work/prepared/hidden/make_source_control.py /work/tmp/control-fixed /work/tmp/source-control-fixed
```

Exact harness test commands, run by `run_logged.py` with separate target directories `/work/tmp/target-source-baseline` and `/work/tmp/target-source-fixed`:

```sh
cd /work/tmp/source-control-baseline
cargo test --offline --test heldback_truncate_upstream -- --exact truncate_invalid --nocapture
cargo test --offline --test heldback_truncate_spec -- --nocapture
cd /work/tmp/source-control-fixed
cargo test --offline --test heldback_truncate_upstream --test heldback_truncate_spec -- --nocapture
cargo test --offline --lib index_map::tests::
cargo check --offline --lib --no-default-features
```

The native baseline commands are the same per-target test commands, plus `cargo test --offline --lib` and `cargo check --offline --lib --no-default-features`, run from `/work/tmp/control-baseline` with target directory `/work/tmp/target-native-baseline`. The fixed native command is the upstream per-target command from `/work/tmp/control-fixed` with target directory `/work/tmp/target-native-fixed`. All complete environments are in the logs.

Development-only logs under `logs/development/` retain an initial neighboring-test harness compile error (incorrect constructor choice). The final test uses the existing public `Default` API on both snapshots. Those development results are superseded by the final logs above and are not behavioral control outcomes. The upstream assertion body was never altered.

### End-to-end scoring-script self-check

Executed the exact scoring workflow on the untouched baseline, not on a treatment:

```sh
python3 /work/prepared/hidden/score_candidate.py /work/crate /work/tmp/scoring-baseline-selfcheck
```

As expected, the script exits 1 after recording all four checks. Its original/default suite passes (186 library tests, 9 integration tests, and 177 doctests), and the library check passes. The upstream and neighboring commands each return 101 for the intended lookup assertion failure. The script continues after the oracle failure and verifies hidden file integrity. Full command output is retained in `logs/scoring-script-baseline-selfcheck.log`; the per-check logs and `results.json` are also in `/work/tmp/scoring-baseline-selfcheck/`. No candidate fix or treatment run was performed.
