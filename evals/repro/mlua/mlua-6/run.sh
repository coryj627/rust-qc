#!/bin/sh
# Exit 0 when create_thread and reset run an unrelated local function instead of the one passed.
cd /work/repro/mlua-6 || exit 2
cargo build --release --offline 2>&1 | tail -1
B=/target/release/repro-mlua-6
fail=0
for m in create_thread reset; do
  $B $m > /tmp/o 2>&1; code=$?
  cat /tmp/o; echo "mode=$m exit=$code"
  grep -q 'Ok("LOCAL function ran")' /tmp/o || fail=1
done
$B out_of_range > /tmp/o 2>&1; code=$?
cat /tmp/o; echo "mode=out_of_range exit=$code"
grep -q 'attempt to call a nil value' /tmp/o || fail=1
echo "== ASan: looking for a memory error in the same modes"
RUSTFLAGS="-Zsanitizer=address" cargo +nightly-2026-10-01 build --release --target aarch64-unknown-linux-gnu --offline 2>&1 | tail -1
export ASAN_OPTIONS=detect_leaks=0
for m in create_thread reset out_of_range; do
  /target/aarch64-unknown-linux-gnu/release/repro-mlua-6 $m > /tmp/o 2>&1; code=$?
  echo "mode=$m exit=$code asan_reports=$(grep -c AddressSanitizer /tmp/o)"
done
[ $fail = 0 ] && echo "DEMONSTRATED: the wrong function runs, with no memory error seen"
exit $fail
