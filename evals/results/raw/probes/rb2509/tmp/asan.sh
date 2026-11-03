#!/bin/sh
cd /work/tmp/probes
export RUSTFLAGS="-Zsanitizer=address" CC=clang CFLAGS="-fsanitize=address -fno-omit-frame-pointer -g"
export ASAN_OPTIONS=detect_leaks=0:halt_on_error=1
cargo +nightly-2026-10-01 build --offline --target aarch64-unknown-linux-gnu --target-dir /target/asan 2>&1 | grep -E '^error|Finished|could not' -A6 | head -30
B=/target/asan/aarch64-unknown-linux-gnu/debug/probes
export HOME=/work/tmp/home TMPDIR=/tmp
for x in "$@"; do
  echo "=== $x"
  timeout 90 $B $x > /tmp/o.txt 2>&1; rc=$?
  grep -v '^$' /tmp/o.txt | grep -E 'ERROR: AddressSanitizer|^(READ|WRITE) of size|freed by thread|previously allocated|#[0-3] .* in |progress|hide cb|delta|probe finished|panicked|precondition|SUMMARY' | cut -c1-170 | head -22
  echo "--- exit=$rc"
done
