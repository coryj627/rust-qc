#!/bin/sh
. /work/repro/lib.sh
prepare moka-1
cargo build --offline --quiet --release --bins || exit 2
T=${CARGO_TARGET_DIR:-/target}/release
echo "--- plain thread stress (report only) ---"
timeout 120 "$T/stress" 6
echo "--- forced stale op, growth measurement (decides the exit code) ---"
timeout 120 "$T/forced"
