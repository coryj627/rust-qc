These controls were authored solely from /work/crate (just 1.50.0), its tests and documentation, and the exclusion list /work/known-defects.json. None of the seven controls relabels an entry in that list. All concern ordinary CLI functional behavior.

The original crate was copied without modifications to /work/tmp/just-controls. Build command, run from that scratch directory:

    RUSTUP_AUTO_INSTALL=0 CARGO_NET_OFFLINE=true CARGO_TARGET_DIR=/target cargo build --locked --bin just

Build output is in build.log. Environment and binary identity are in environment.json. baseline-sha256.json records the original tree before the build; validate.py verifies that both the original and scratch trees still have identical baseline files.

Reproduce all 23 small regression cases:

    python3 /work/prepared/evidence/probe.py

Or pass any control ID to probe.py to run that control's cases. Each control directory contains its justfile fixture(s) and probe-log.json, including argv, working directory, controlled environment changes, exit code, exact stdout/stderr, expected result, and pass/fail. The runner removes JUST_* variables, disables dotenv, closes stdin, and gives each subprocess a 10-second timeout. All commands use only the local binary and ordinary local shell builtins, or evaluate expressions.

source-excerpts.txt preserves line-numbered refutation code and documentation from the original source. controls.json's commands assume the scratch binary has been built as above.

Validation:

    python3 /work/prepared/evidence/validate.py

Limits: tested on aarch64 Linux using the supplied stable Rust 1.99.1. These are targeted functional probes, not the full cargo test suite. No platform-specific Windows behavior was exercised. No dedicated security testing was performed.
