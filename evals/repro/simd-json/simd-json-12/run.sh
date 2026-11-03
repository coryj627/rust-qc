#!/bin/sh
# Backend: NEON; number parsing is shared code. Exit 42 from the binary means a wrong number was returned.
export RUSTFLAGS="--cap-lints allow"
cd /work/repro/simd-json-12 || exit 2
cargo build --offline -q || exit 2
uname -m
/target/debug/repro-simd-json-12; rc=$?
echo "exit=$rc"
[ $rc -eq 42 ]
