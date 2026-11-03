#!/bin/sh
# Fix control and attribution for moka-3. Usage: control.sh <tree under /work/fix>
. /work/repro/lib.sh
MOKA_DIR=/work/fix/$1
prepare moka-3
MIRIFLAGS="-Zmiri-tree-borrows -Zmiri-ignore-leaks -Zmiri-permissive-provenance" timeout 240 cargo +nightly-2026-10-01 miri run --offline --quiet --bin forced > /tmp/m.out 2>&1
echo "miri forced exit=$?"; grep -E "hook fired|finished|Undefined Behavior" /tmp/m.out | cut -c1-150 | head -6
grep -m3 -E "at /work/fix/.*(timer_wheel|base_cache).rs" /tmp/m.out | cut -c1-110
cargo build --offline --quiet --release --bins || exit 2
T=${CARGO_TARGET_DIR:-/target}/release
for i in 1 2 3; do "$T/forced" > /tmp/n.out 2>&1; echo "native forced run $i exit=$? $(grep -m1 -E 'panicked at|free\(\)|corrupted' /tmp/n.out | cut -c1-110)"; done
hits=0; i=0
while [ "$i" -lt 6 ]; do i=$((i+1)); "$T/stress" 5 > /tmp/s.out 2>&1; e=$?; [ "$e" -ne 0 ] && hits=$((hits+1)); echo "stress run $i exit=$e $(grep -m1 -E 'panicked at|free\(\)|corrupted|malloc' /tmp/s.out | cut -c1-110)"; done
echo "plain stress runs with a crash or panic: $hits of $i"
