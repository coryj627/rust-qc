#!/bin/sh
set -u
export RUSTUP_AUTO_INSTALL=0
export CARGO_NET_OFFLINE=true
export CARGO_TARGET_DIR=/work/tmp/small-config-submission-target
crate_dir=${1:-/work/crate}
log_dir=${2:-/work/tmp/small-config-scoring}
hidden_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
mkdir -p "$log_dir"
: > "$log_dir/status.tsv"
run_check() {
    label=$1
    shift
    if "$@" > "$log_dir/$label.log" 2>&1; then
        status=0
    else
        status=$?
    fi
    printf '%s\t%s\n' "$label" "$status" >> "$log_dir/status.tsv"
}
run_check library cargo check --manifest-path "$crate_dir/Cargo.toml" --offline --lib
run_check submitted-tests cargo test --manifest-path "$crate_dir/Cargo.toml" --offline --all-targets
run_check doctests cargo test --manifest-path "$crate_dir/Cargo.toml" --offline --doc
run_check public-api "$hidden_dir/run_api_tests.sh" "$crate_dir"
run_check fmt cargo fmt --manifest-path "$crate_dir/Cargo.toml" --check
run_check clippy cargo clippy --manifest-path "$crate_dir/Cargo.toml" --offline --all-targets
cat "$log_dir/status.tsv"
# This orchestrator exits successfully after collecting checks, even on failures.
# Individual check outcomes are in status.tsv and must be reviewed.
