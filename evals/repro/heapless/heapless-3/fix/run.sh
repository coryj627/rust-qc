#!/bin/sh
cd /work/repro/heapless-3/fix || exit 2
log=/tmp/heapless-3-fix.log
MIRIFLAGS=-Zmiri-ignore-leaks cargo +nightly-2026-10-01 miri run -q >$log 2>&1
sed -n 1,40p $log
grep -q 'Undefined Behavior' $log
