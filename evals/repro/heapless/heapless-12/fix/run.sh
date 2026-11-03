#!/bin/sh
cd /work/repro/heapless-12/fix || exit 2
log=/tmp/heapless-12-fix.log
cargo run -q >$log 2>&1; rc=$?
cat $log; echo "program exit $rc"
grep -q 'len() must count the 2 queued items' $log
