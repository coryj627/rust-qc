#!/bin/sh
# Exits 0 only if a non-zero union field offset is seen. rustc gives every union field offset 0,
# so this is expected to exit 1: the defect is a missing language guarantee, not a visible failure.
cd /work/repro/heapless-7/fix || exit 2
status=1
echo "=== stable default layout"
cargo run -q; rc=$?; echo "exit $rc"; [ $rc = 0 ] && status=0
for seed in 1 2 3 4; do
  echo "=== nightly -Zrandomize-layout -Zlayout-seed=$seed"
  CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-/target}/randomize-$seed RUSTFLAGS="-Zrandomize-layout -Zlayout-seed=$seed" cargo +nightly-2026-10-01 run -q; rc=$?
  echo "exit $rc"; [ $rc = 0 ] && status=0
done
exit $status
