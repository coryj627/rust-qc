#!/bin/sh
# Reproducer for qp-655a8ad. Exits 0 iff the bug is demonstrated at the baseline.
# Isolated target dir avoids collision with any other source tree sharing /target.
set -e
cargo test --locked --manifest-path /work/reprotree/Cargo.toml --target-dir /work/reprotree/target -p quinn-proto repro_key_655a8ad -- --nocapture 2>&1 | tee /tmp/qp-655a8ad.out
grep -q "test result: ok. 1 passed" /tmp/qp-655a8ad.out
echo "DEMONSTRATED qp-655a8ad"
