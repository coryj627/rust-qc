#!/bin/sh
# Default Miri does not validate behind references, so it stays silent. The report comes only with
# -Zmiri-recursive-validation, which treats a reference to invalid data as UB.
cd /work/repro/heapless-10 || exit 2
log=/tmp/heapless-10.log
echo "=== miri, default flags (expected silent)"
MIRIFLAGS=-Zmiri-ignore-leaks cargo +nightly-2026-10-01 miri run -q >$log 2>&1; echo "exit $?"
grep -E 'Undefined Behavior|no UB' $log
echo "=== miri, -Zmiri-recursive-validation"
MIRIFLAGS="-Zmiri-ignore-leaks -Zmiri-recursive-validation" cargo +nightly-2026-10-01 miri run -q >$log 2>&1; echo "exit $?"
sed -n 1,30p $log
grep -q 'Undefined Behavior' $log
