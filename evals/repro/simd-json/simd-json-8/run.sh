#!/bin/sh
# Backend: NEON stage 1 and parse_str; get_unicode_codepoint is shared code. Exit 42 means NUL results were observed.
export RUSTFLAGS="--cap-lints allow"
cd /work/repro/simd-json-8 || exit 2
cargo build --offline -q || exit 2
uname -m
/target/debug/repro-simd-json-8; rc=$?
echo "exit=$rc"
[ $rc -eq 42 ]
