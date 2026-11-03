#!/bin/sh
# Not public-API evidence: an in-crate test on a scratch copy, because of the 2^63 dequeues.
src=/work/base
log=/tmp/heapless-13.log
w=/tmp/heapless-13-src
mkdir -p $w && cp -R $src/. $w/
cat /work/repro/heapless-13/key_tests.rs >> $w/src/spsc.rs
cd $w || exit 2
cargo test -q --lib --features=portable-atomic-critical-section key_tests >$log 2>&1; rc=$?
grep -vE '^warning|^\s*(\||-->|=)|^\s*[0-9]+ \|' $log | sed -n 1,25p; echo "test exit $rc"
grep -q 'attempt to add with overflow' $log
