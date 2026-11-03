These controls were authored against /work/crate/quinn-proto version 0.11.13.
Only the supplied baseline and /work/known-defects.json were consulted.
The two decoys and five findings describe seven distinct false defects;
refutations in controls.json contain both source guards and local probe results.
None reuses the supplied known defects. In particular, decoy-2 uses RangeSet::insert,
not the defective unordered-read RangeSet::replace path, and false-5 uses a live
receive stream without stop, reset, or an empty STREAM payload.

Executed commands (working directory /work/tmp/control-probe):

    RUSTUP_AUTO_INSTALL=0 cargo +1.99.1 test --locked -p quinn-proto negative_control_probes -- --nocapture
    RUSTUP_AUTO_INSTALL=0 cargo +1.99.1 test --locked -p quinn-proto

The first command passed 7 focused probes (263 other unit tests filtered out).
The second passed 270 unit tests (263 baseline + 7 probes) and 3 doc tests.
probes.log and crate-tests.log contain actual command output. An initial attempt
using +stable failed before compilation because the toolchain alias was absent;
it is retained in toolchain-alias-attempt.log. The installed explicit +1.99.1
was then used successfully. Nothing was installed.

probes/ preserves exactly the test modules appended to six files in the scratch
copy. probe-files.txt lists those files. Production source was not edited in
either tree: scratch files are baseline bytes followed by these test modules.
reproduce.sh is a convenience recipe for recreating that same setup in a new
scratch directory; this recipe itself was not separately executed.

baseline.sha256 covers every original crate file before probe preparation.
validate.py checks the JSON structure, IDs, counts, baseline file contents,
source-reference line numbers and exact scratch-copy modifications.
validation.log records its execution. source-context.txt retains numbered
original source excerpts for every control's guard.

Scope: local Rust 1.99.1 tests on aarch64 Linux using default crate features.
Probes call the production parser, buffer, estimator, token validator, MTU state
machine, path-response queue and receive-stream state directly inside unit tests.
The full existing suite additionally exercises connection-level behavior.
No network traffic, external behavior, Miri runs or feature-matrix claims.
