#!/bin/sh
# Backend: NEON (aarch64); the recursion is in shared code. Shows a stack-overflow abort, dev and release.
export RUSTFLAGS="--cap-lints allow"
cd /work/repro/simd-json-1 || exit 2
cargo build --offline -q || exit 2
cargo build --offline -q --release || exit 2
uname -m
fail=0
for prof in debug release; do
  B=/target/$prof/repro-simd-json-1
  echo "== $prof"
  $B tape 1000000 || fail=1
  $B owned 1000 || fail=1
  for m in owned borrowed; do
    $B $m 1000000 > /tmp/o.txt 2>&1; rc=$?
    cat /tmp/o.txt; echo "$m depth=1000000 (2 MB input) exit=$rc"
    if [ $rc -eq 134 ] && grep -q "overflowed its stack" /tmp/o.txt; then :; else fail=1; fi
  done
done
exit $fail
