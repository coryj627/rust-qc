# Author validation of the frozen baseline

Only the supplied workspace was inspected. No fix, candidate, upstream patch, external website, skill, or evaluation result was used. Tests were added only to disposable source copies. The production workspace `/work/crate` remains unchanged.

Source identity: `alacritty_terminal/Cargo.toml` declares version 0.25.0; there is no Git checkout metadata. `baseline-files.sha256` records all 324 original regular files. `baseline-source.tar.gz` preserves the supplied workspace, including symlinks, without assessor tests. `baseline-integrity.log` records verification of the live original; the runner's pre-injection `candidate.diff` is empty for this baseline.

Environment: installed `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, `cargo 1.99.1 (5f94df478 2026-08-27)`, host `aarch64-unknown-linux-gnu`. The runner uses `/work/.cargo-home`, `RUSTUP_AUTO_INSTALL=0`, `--locked --offline`, and executable scratch/target files under `/work/tmp`. No graphical application was built and no toolchain or dependency was installed.

The final runner was validated with this exact `sandbox_exec` shell command:

```sh
sh /work/prepared/hidden/run.sh /work/crate /work/prepared/evidence/runner-baseline
```

The shell returned **1**, as expected for a baseline with the specified defect. Its individual Cargo outcomes are preserved in `runner-baseline/status.tsv` and the following logs. This is baseline author validation, not treatment grading.

| Check / log basename | Exit | Observed result |
| --- | ---: | --- |
| `hidden-compile-default` | 0 | Public-API integration target compiles |
| `hidden-default` | 101 | 23 executed: 10 pass, 13 fail |
| `hidden-compile-no-default` | 0 | Same target compiles without default features |
| `hidden-no-default` | 101 | Same 10 passes and 13 failures |
| `existing-lib-default` | 0 | 128 pass |
| `existing-ref-default` | 0 | 44 pass |
| `docs-default` | 0 | 1 pass |
| `candidate-target-inventory` | 0 | Metadata query succeeds; no additional baseline integration targets |
| `existing-lib-no-default` | 0 | 127 pass |

Ten hidden failures directly expose the requested defects: setting/resetting DECOM does not home or clear pending wrap (six tests), and saved-on/saved-off restoration does not restore mode or subsequent coordinate addressing (four tests). Saved absolute positions still restore correctly (two passing core tests). Eight neighboring controls pass, covering existing margin, scroll, rendition/charset, wrap, and unrelated-mode behavior. Three further failures cover full-screen homing, overwriting/reusing saved state, and default saved origin state after full reset. See the test source and rubric for exact names; no expected result was taken from a reference fix.

The runner records a pre-injection diff, refuses to overwrite a candidate file with the hidden target's name, captures all named check exit codes independently, discovers and executes candidate-added integration targets, and cleans its temporary source and target directory on exit. Initial exploratory copies and their hidden test binaries were also removed; only the assessor bundle retains hidden test material. The runner passed `sh -n`.

Limits: no known-good implementation was written or run. Baseline compilation, expected assertion failures, and passing neighboring/existing tests establish a usable defect case but cannot prove that every acceptable repair will pass. The suite does not cover every terminal size, alias, lifecycle, renderer behavior, platform, downstream caller, or minimum Rust version. Treatment isolation must be enforced by the provisioning runner as described in `scoring.md`; this authoring filesystem is not itself a secrecy boundary.
