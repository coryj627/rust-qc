# Task

Fix the malformed-image-wikilink panic in pulldown-cmark 0.13.0. With wikilinks enabled, malformed link delimiters can produce subtraction overflow in debug builds and an invalid text slice in release builds. Parsing untrusted text must handle these malformed links without panicking, while preserving valid wikilink and ordinary Markdown behavior. Keep public APIs stable and avoid unrelated changes. Add regression coverage.

# Workspace and build

The editable baseline repository is `/work/crate`. It is a Rust workspace; the parser package is `pulldown-cmark` and its companion escaping package is local to the workspace. Use the installed Rust toolchain. The declared minimum Rust version is 1.71.1; do not install a toolchain or dependencies. Network access is disabled and baseline dependencies are cached under `/work/.cargo-home`.

Run commands from `/work/crate`, with:

```sh
export CARGO_HOME=/work/.cargo-home
export CARGO_NET_OFFLINE=true
export RUSTUP_AUTO_INSTALL=0
export CARGO_TARGET_DIR=/work/tmp/candidate-target
cargo test --offline --locked -p pulldown-cmark
cargo test --offline --locked -p pulldown-cmark --release
cargo check --offline --locked -p pulldown-cmark --no-default-features
```

Use `/work/tmp` for executable temporary files and build output. Keep the repository's existing APIs and feature combinations usable. Add regression tests to the normal repository test suite. Inspect the existing parser, documentation, and tests as needed. Do not enable `gen-tests` merely to run existing tests: that feature regenerates checked-in suites.

Only this task sheet and the editable baseline should be exposed to the treatment agent. Author-only validation material, hidden tests, scoring instructions, logs, and the fixed snapshot must be withheld.
