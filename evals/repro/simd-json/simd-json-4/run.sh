#!/bin/sh
# Release build: timing of duplicate vs distinct keys at doubling sizes (halfbrown 0.3.0 from Cargo.lock).
export RUSTFLAGS="--cap-lints allow"
cd /work/repro/simd-json-4 || exit 2
cargo build --offline --release -q || exit 2
grep -A1 'name = "halfbrown"' Cargo.lock | tr '\n' ' '; echo
grep -A1 'name = "hashbrown"' Cargo.lock | tr '\n' ' '; echo
uname -m
/target/release/repro-simd-json-4
rc=$?
echo "exit=$rc"
exit $rc
