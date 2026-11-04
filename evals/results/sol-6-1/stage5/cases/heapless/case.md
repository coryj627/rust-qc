# Task

Fix IndexMap::truncate in heapless 0.9.1. After truncating a map containing colliding hashes, every retained entry must remain reachable by get and contains_key, with iteration order and length consistent. Preserve retained values and behavior for zero, intermediate, equal and larger truncation lengths. Keep the public API stable and avoid unrelated changes. Add regression coverage.

# Workspace and build

The working repository is `/work/crate`, the heapless 0.9.1 baseline. Make your changes there. This is a Rust 2021 crate designed for fixed-capacity, allocation-free use, including `no_std`.

Use the already installed Rust toolchain and cached dependencies. Network access is disabled; do not install tools or toolchains. Use `/work/tmp` for executable temporary files and build artifacts. Run commands through `sandbox_exec`.

```sh
cd /work/crate
export CARGO_HOME=/work/.cargo-home
export CARGO_TARGET_DIR=/work/tmp/candidate-target
export CARGO_NET_OFFLINE=true
export RUSTUP_AUTO_INSTALL=0
cargo test --offline --lib index_map::tests::
cargo test --offline
cargo check --offline --lib --no-default-features
```

Add your own regression coverage and report your changes and verification results. Preserve existing behavior outside the requested fix. Hidden evaluation materials and the fixed control are not part of the treatment workspace.

# Provenance

Project: heapless (`rust-embedded_heapless.git`). Case ID: `heapless`; bug ID: `heapless-6`. Task wording comes verbatim from `/work/case-input.json`. Task source: `src/index_map.rs` in the 0.9.1 baseline. The evaluator privately records the upstream commit and oracle provenance; no upstream patch or held-back tests are supplied with this task.
