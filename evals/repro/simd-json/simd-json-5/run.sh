#!/bin/sh
# Backend: NEON; tape accessors are shared code. Exit 42 from the binary means wrong results were observed.
export RUSTFLAGS="--cap-lints allow"
cd /work/repro/simd-json-5 || exit 2
cargo build --offline -q || exit 2
uname -m
/target/debug/repro-simd-json-5; rc=$?
echo "exit=$rc"
[ $rc -eq 42 ]
