#!/bin/sh
# Panicking Drop during clear(): Miri reports UB for each of the three containers.
cd /work/repro/heapless-1/fix || exit 2
log=/tmp/heapless-1-fix.log
status=0
for mode in deque history_buf; do
  echo "=== mode $mode (miri)"
  MIRIFLAGS=-Zmiri-ignore-leaks cargo +nightly-2026-10-01 miri run -q -- $mode >$log 2>&1
  sed -n 1,30p $log
  grep -q 'Undefined Behavior' $log || status=1
done
# --release: a debug_assert would otherwise stop the stale read in the debug profile
echo "=== mode index_map (miri, release, no debug assertions)"
MIRIFLAGS=-Zmiri-ignore-leaks cargo +nightly-2026-10-01 miri run -q --release -- index_map >$log 2>&1
sed -n 1,30p $log
grep -q 'Undefined Behavior' $log || status=1
exit $status
