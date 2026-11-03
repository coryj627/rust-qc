#!/bin/sh
# Exit 0 when ASan reports heap-use-after-free for both defects from safe code.
cd /work/repro/mlua-4 || exit 2
RUSTFLAGS="-Zsanitizer=address" cargo +nightly-2026-10-01 build --release --target aarch64-unknown-linux-gnu --offline 2>&1 | tail -1
A=/target/aarch64-unknown-linux-gnu/release/repro-mlua-4
export ASAN_OPTIONS=detect_leaks=0
fail=0
for m in method teardown; do
  $A $m > /tmp/o 2>&1; code=$?
  echo "== ASan mode=$m exit=$code"
  grep -vE '^ +#[0-9]+ ' /tmp/o | cut -c1-160 | head -14
  grep -q "AddressSanitizer: heap-use-after-free" /tmp/o || fail=1
done
echo "== without ASan (stable, release): informational"
cargo build --release --offline 2>&1 | tail -1
for m in method teardown; do
  /target/release/repro-mlua-4 $m 2>&1 | cut -c1-160; echo "mode=$m"
done
[ $fail = 0 ] && echo "DEMONSTRATED: heap-use-after-free in both modes under ASan"
exit $fail
