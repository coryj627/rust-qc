#!/bin/sh
# Reproducer for qp-fed0321. Exits 0 iff the bug is demonstrated at the baseline.
# Isolated target dir avoids collision with any other source tree sharing /target.
set -e
cargo test --locked --manifest-path /work/reprotree/Cargo.toml --target-dir /work/reprotree/target -p quinn-proto repro_key_fed0321 -- --nocapture 2>&1 | tee /tmp/qp-fed0321.out
grep -q "test result: ok. 1 passed" /tmp/qp-fed0321.out
echo "DEMONSTRATED qp-fed0321"
