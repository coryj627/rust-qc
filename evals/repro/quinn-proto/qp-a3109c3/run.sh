#!/bin/sh
# Reproducer for qp-a3109c3. Exits 0 iff the bug is demonstrated at the baseline.
# Isolated target dir avoids collision with any other source tree sharing /target.
set -e
cargo test --locked --manifest-path /work/reprotree/Cargo.toml --target-dir /work/reprotree/target -p quinn-proto repro_key_a3109c3 -- --nocapture 2>&1 | tee /tmp/qp-a3109c3.out
grep -q "test result: ok. 1 passed" /tmp/qp-a3109c3.out
echo "DEMONSTRATED qp-a3109c3"
