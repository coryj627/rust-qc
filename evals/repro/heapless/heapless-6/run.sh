#!/bin/sh
cd /work/repro/heapless-6 || exit 2
log=/tmp/heapless-6.log
cargo run -q >$log 2>&1; rc=$?
cat $log; echo "program exit $rc"
grep -q "key 2 is in iter() but get() cannot find it" $log
