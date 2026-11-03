#!/bin/sh
cd /work/repro/heapless-4/fix || exit 2
log=/tmp/heapless-4-fix.log
echo "=== stable, debug build (library UB check)"
cargo run -q >$log 2>&1; echo "exit $?"; sed -n 1,12p $log
echo "=== stable, release build (no UB checks: the safe API goes past capacity)"
cargo run -q --release >$log 2>&1; echo "exit $?"; sed -n 1,12p $log
echo "=== miri, release"
MIRIFLAGS=-Zmiri-ignore-leaks cargo +nightly-2026-10-01 miri run -q --release >$log 2>&1
sed -n 1,30p $log
grep -q 'Undefined Behavior' $log
