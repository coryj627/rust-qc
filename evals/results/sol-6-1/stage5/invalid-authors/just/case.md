# Task

Fix just 1.50.0 so --dry-run never executes commands passed to the shell() function. Dry-run output should represent the unevaluated shell call clearly, including properly quoted and escaped arguments. Preserve actual execution outside dry-run and existing backtick behavior. Keep public behavior otherwise stable, avoid unrelated changes, and add regression coverage.

# Workspace and build

The baseline project is `/work/crate`, a Rust workspace whose root package is `just` 1.50.0. Work in that project. The integration test target is named `integration` and is rooted at `tests/lib.rs`; integration test modules must be registered there. Follow existing project conventions when adding regression coverage.

Use the already installed default Rust toolchain and the cached dependencies. Network access is off. Do not install tools or toolchains. Use `sandbox_exec` for all file and command operations. Executable temporary files belong under `/work/tmp`. No skills are available.

Use these environment settings for builds and tests:

```sh
export RUSTUP_AUTO_INSTALL=0
export CARGO_HOME=/work/.cargo-home
export CARGO_TARGET_DIR=/work/tmp/candidate-target
export TMPDIR=/work/tmp
export USER=just-case
export XDG_RUNTIME_DIR=/work/tmp/runtime
mkdir -p "$TMPDIR/runtime"
cd /work/crate
cargo build --offline --locked --bin just
cargo test --offline --locked --test integration functions::
cargo test --offline --locked --test integration backticks::
cargo test --offline --locked --test integration misc::dry_run -- --exact
```

Run the relevant existing tests and your added regression tests. A full integration run is optional if practical; report any environmental failures precisely. Keep the patch focused on the requested public behavior.

# Provenance

Case `just-6`, project `just`, source repository `casey_just.git`. The supplied reference revision is `0beb79f49def4245757cddf15d52e485332ecb49`; it is retained only as grading provenance. The upstream regression source is `tests/functions.rs`. No reference source or held-back grading material is part of the treatment handoff.
