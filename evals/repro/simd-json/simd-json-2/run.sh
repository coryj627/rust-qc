#!/bin/sh
# Native dev build (NEON): index panic. Miri, x86_64 target, release profile (debug assertions off,
# native fallback backend because Miri reports no AVX2/SSE4.2): UB in next_. Copy of the baseline with
# only the line `#![deny(warnings)]` removed, because cargo-miri rejects --cap-lints in RUSTFLAGS.
export TMPDIR=/target/tmp; mkdir -p $TMPDIR /target/nolint-base
cp -r /work/base/. /target/nolint-base/ && sed -i '1{/deny(warnings)/d}' /target/nolint-base/src/lib.rs
cd /work/repro/simd-json-2 || exit 2
fail=0
echo "== native dev, $(uname -m), NEON"
cargo run --offline -q > /tmp/n.txt 2>&1; rc=$?; sed -n '/^first = /,$p' /tmp/n.txt; echo "exit=$rc"
grep -q "index out of bounds: the len is 1 but the index is 1" /tmp/n.txt || fail=1
echo "== miri, x86_64 target, release profile, native fallback backend"
MIRI_SYSROOT=/work/miri-sysroot-x86 cargo +nightly-2026-10-01 miri run --offline -q --release --target x86_64-unknown-linux-gnu > /tmp/m.txt 2>&1; rc=$?
sed -n '/^first = /,$p' /tmp/m.txt; echo "exit=$rc"
grep -q "Undefined Behavior" /tmp/m.txt || fail=1
grep -q "simd_json::Deserializer::<'_>::next_" /tmp/m.txt || fail=1
exit $fail
