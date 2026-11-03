#!/bin/sh
# Native dev build (NEON): index panic. Miri, x86_64 target, release profile, native fallback backend: UB.
# Copy of the baseline with only `#![deny(warnings)]` removed.
export TMPDIR=/target/tmp; mkdir -p $TMPDIR /target/nolint-base
cp -r /work/base/. /target/nolint-base/ && sed -i '1{/deny(warnings)/d}' /target/nolint-base/src/lib.rs
cd /work/repro/simd-json-10 || exit 2
fail=0
echo "== native dev, $(uname -m), NEON"
cargo run --offline -q > /tmp/n.txt 2>&1; rc=$?; sed -n '/^thread\|^result/,$p' /tmp/n.txt; echo "exit=$rc"
grep -q "index out of bounds" /tmp/n.txt || fail=1
echo "== miri, x86_64 target, release profile, native fallback backend"
MIRI_SYSROOT=/work/miri-sysroot-x86 cargo +nightly-2026-10-01 miri run --offline -q --release --target x86_64-unknown-linux-gnu > /tmp/m.txt 2>&1; rc=$?
sed -n '/^error: Undefined\|^result/,$p' /tmp/m.txt; echo "exit=$rc"
grep -q "Undefined Behavior" /tmp/m.txt || fail=1
exit $fail
