#!/bin/sh
# Miri, x86_64 target: Miri reports no AVX2/SSE4.2, so is_x86_feature_detected takes the native fallback.
# Copy of the baseline with only `#![deny(warnings)]` removed (cargo-miri rejects --cap-lints in RUSTFLAGS).
export TMPDIR=/target/tmp; mkdir -p $TMPDIR /target/nolint-base
cp -r /work/base/. /target/nolint-base/ && sed -i '1{/deny(warnings)/d}' /target/nolint-base/src/lib.rs
cd /work/repro/simd-json-3 || exit 2
fail=0
echo "== native dev, $(uname -m), NEON (validated, for contrast)"
cargo run --offline -q 2>/dev/null; echo "exit=$?"
echo "== miri, x86_64 target, native fallback backend"
MIRI_SYSROOT=/work/miri-sysroot-x86 cargo +nightly-2026-10-01 miri run --offline -q --target x86_64-unknown-linux-gnu > /tmp/m.txt 2>&1; rc=$?
sed -n '/^ACCEPTED\|^rejected/,$p' /tmp/m.txt; echo "exit=$rc"
grep -q 'ACCEPTED bytes=\[255\] from_utf8_ok=false' /tmp/m.txt || fail=1
grep -q "Undefined Behavior" /tmp/m.txt || fail=1
exit $fail
