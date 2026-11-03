#!/bin/sh
cd /work/repro/heapless-8 || exit 2
log=/tmp/heapless-8.log
echo "=== stable, debug (debug_assert in the index lookup)"
cargo run -q >$log 2>&1; echo "exit $?"; sed -n 1,12p $log
# Vec::retain, used inside IndexMap::retain, has its own Stacked Borrows error on this same panic
# path (bug heapless-9). Aliasing checks are off here so that Miri reaches this bug.
echo "=== miri, release (no debug assertions), -Zmiri-disable-stacked-borrows"
MIRIFLAGS="-Zmiri-ignore-leaks -Zmiri-disable-stacked-borrows" cargo +nightly-2026-10-01 miri run -q --release >$log 2>&1
sed -n 1,30p $log
grep -q 'Undefined Behavior' $log
