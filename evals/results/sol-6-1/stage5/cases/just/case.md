# Task

Fix just 1.50.0 so --dry-run never executes commands passed to the shell() function. Dry-run output should represent the unevaluated shell call clearly, including properly quoted and escaped arguments. Preserve actual execution outside dry-run and existing backtick behavior. Keep public behavior otherwise stable, avoid unrelated changes, and add regression coverage.

# Workspace and build

Work in `/work/crate`, the just 1.50.0 baseline. Implement the change there and add focused regression tests using the existing test conventions. Keep the project version and unrelated public behavior stable.

The crate uses Rust edition 2024 and declares Rust 1.85.0 as its minimum version. An installed Rust/Cargo toolchain is available; do not install tools or toolchains. Network access is disabled. Use the existing offline dependency cache:

```sh
cd /work/crate
export CARGO_HOME=/work/.cargo-home
export CARGO_TARGET_DIR=/work/tmp/treatment-target
export RUSTUP_AUTO_INSTALL=0
export TMPDIR=/work/tmp
cargo build --offline --locked -p just --bin just
cargo test --offline --locked -p just --test integration functions::
cargo test --offline --locked -p just --test integration misc::dry_run -- --exact
```

The integration target is declared explicitly in Cargo.toml and starts at `tests/lib.rs`. Register any new integration module there, or extend an existing module. The test harness invokes the built executable and generally uses Bash. Existing shell, backtick, and dry-run checks can be run with additional filters. Use `/work/tmp` for executable temporary files and build artifacts. Do not change files outside the candidate workspace except temporary build artifacts.

# Provenance

Project: `casey_just.git`; bug ID: `just-6`. Task text comes verbatim from `/work/case-input.json`. Source baseline: `/work/crate` (package version 1.50.0). Upstream reference commit: `0beb79f49def4245757cddf15d52e485332ecb49`. Hidden evaluation materials are withheld.
