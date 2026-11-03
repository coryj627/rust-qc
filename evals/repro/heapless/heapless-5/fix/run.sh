#!/bin/sh
cd /work/repro/heapless-5/fix || exit 2
log=/tmp/heapless-5-fix.log
echo "=== stable, debug, default FNV hasher (library UB check)"
cargo run -q -- fnv >$log 2>&1; echo "exit $?"; sed -n 1,10p $log
echo "=== stable, release, default FNV hasher (no UB check)"
cargo run -q --release -- fnv >$log 2>&1; echo "exit $?"; sed -n 1,10p $log
echo "=== miri, release, identity hasher (no library UB check, so Miri sees the bad value)"
MIRIFLAGS=-Zmiri-ignore-leaks cargo +nightly-2026-10-01 miri run -q --release -- identity >$log 2>&1
sed -n 1,30p $log
grep -q 'Undefined Behavior' $log
