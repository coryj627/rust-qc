#!/bin/sh
set -eu
export RUSTUP_AUTO_INSTALL=0
export CARGO_NET_OFFLINE=true
crate_dir=${1:-/work/crate}
case "$crate_dir" in
    /*) ;;
    *) echo 'usage: run_api_tests.sh /absolute/path/to/crate' >&2; exit 2 ;;
esac
if [ ! -f "$crate_dir/Cargo.toml" ]; then
    echo "missing Cargo.toml: $crate_dir/Cargo.toml" >&2
    exit 2
fi
hidden_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
mkdir -p /work/tmp
harness_dir=$(mktemp -d /work/tmp/small-config-api.XXXXXX)
trap 'rm -rf "$harness_dir"' EXIT HUP INT TERM
mkdir -p "$harness_dir/tests"
# TOML literal strings support the fixed paths used by these scoring commands.
case "$crate_dir" in *"'"*) echo 'apostrophe in crate path unsupported by runner' >&2; exit 2 ;; esac
cat > "$harness_dir/Cargo.toml" <<MANIFEST
[package]
name = "small_config_public_api_checks"
version = "0.0.0"
edition = "2021"

[dependencies]
small_config = { path = '$crate_dir' }

[workspace]
MANIFEST
cp "$hidden_dir/public_api.rs" "$harness_dir/tests/public_api.rs"
export CARGO_TARGET_DIR=/work/tmp/small-config-api-target
cargo test --manifest-path "$harness_dir/Cargo.toml" --offline --test public_api -- --test-threads=1
