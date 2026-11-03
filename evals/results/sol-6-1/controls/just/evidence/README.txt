Evidence for just 1.50.0 controls

The crate at /work/crate was only read. The build used an unmodified source copy
at /work/tmp/just-control-build, with CARGO_TARGET_DIR=/target and
RUSTUP_AUTO_INSTALL=0. build.log and build-metadata.json record the successful
locked binary build, actual tool versions, and binary SHA-256.

probe.py is the preserved CLI regression harness. Each control has a directory
containing its justfile fixture(s), any .env fixture, and results.json recording
actual argv, environment overrides, stdout, stderr, and exit status. The matching
<id>.log gives concise human-readable results. All 18 cases passed. Commands in
the controls' refutations are the commands actually executed; CLI subprocess
commands are recorded in each results.json and log.

Reproduce any individual control, without editing the crate:
  /usr/bin/python3 /work/prepared/evidence/probe.py decoy-1
Use decoy-2 or false-1 through false-5 as the final argument for the others.
The harness writes only its own evidence directory and runs safe printf commands
or pure expression evaluations. The invalid regex and selected error() sentinel
are expected ordinary exit-1 cases and were checked for the correct diagnostics.

validate.py checks the exact JSON shape, seven required IDs, source references,
all recorded probe results, and all 288 original crate-file SHA-256 values. It
also checks that no file was added to the baseline. validation.log records its
successful run. baseline-before.sha256 is the original complete file manifest.
record_metadata.py verifies the build copy against that same manifest;
metadata.log records success. source-excerpts.txt preserves the cited lines.

The seven mechanisms were reviewed against all 11 entries in known-defects.json;
none restates a known defect. This is a focused control validation, not a full
project audit. No full cargo test suite was run. Execution coverage is Linux
aarch64 with the default POSIX sh; no Windows or custom-shell coverage is claimed.
