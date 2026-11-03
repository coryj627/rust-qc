#!/bin/sh
# Miri, aarch64 target, NEON backend. Symbolic alignment check makes the result independent of where the
# allocation lands. Copy of the baseline with only `#![deny(warnings)]` removed.
export TMPDIR=/target/tmp; mkdir -p $TMPDIR /target/nolint-base
cp -r /work/base/. /target/nolint-base/ && sed -i '1{/deny(warnings)/d}' /target/nolint-base/src/lib.rs
cd /work/repro/simd-json-9 || exit 2
uname -m
MIRIFLAGS=-Zmiri-symbolic-alignment-check cargo +nightly-2026-10-01 miri run --offline -q > /tmp/m.txt 2>&1; rc=$?
sed -n '/^error: Undefined\|^parsed ok/,$p' /tmp/m.txt; echo "exit=$rc"
grep -q "Undefined Behavior" /tmp/m.txt && grep -q "alignment" /tmp/m.txt && grep -q "neon::stage1::SimdInput as simd_json::Stage1Parse>::flatten_bits" /tmp/m.txt
