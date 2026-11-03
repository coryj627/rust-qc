#!/bin/sh
cd /work/repro/heapless-9/fix || exit 2
log=/tmp/heapless-9-fix.log
echo "=== miri, default (Stacked Borrows)"
MIRIFLAGS=-Zmiri-ignore-leaks cargo +nightly-2026-10-01 miri run -q >$log 2>&1
sed -n 1,40p $log
grep -q 'Undefined Behavior' $log; sb=$?
echo "=== miri, -Zmiri-tree-borrows (informational)"
MIRIFLAGS="-Zmiri-ignore-leaks -Zmiri-tree-borrows" cargo +nightly-2026-10-01 miri run -q 2>&1 | grep -E 'Undefined Behavior|no UB'
exit $sb
