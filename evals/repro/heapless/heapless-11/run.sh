#!/bin/sh
cd /work/repro/heapless-11 || exit 2
log=/tmp/heapless-11.log
echo "=== debug build: insert sequential keys into FnvIndexMap<u32, u32, 131072>"
cargo run -q -- where
cargo run -q -- insert >$log 2>&1; rc=$?; echo "exit $rc"; sed -n 1,8p $log
grep -q 'attempt to add with overflow' $log; panic=$?
echo "=== release build: lookups, entries below and above 65536"
cargo run -q --release -- lookup 60000
cargo run -q --release -- lookup 70000
echo "=== release build: one remove, entries below and above 65537"
cargo run -q --release -- remove 1000
cargo run -q --release -- remove 65536
cargo run -q --release -- remove 65537
hang=$?
[ $panic = 0 ] && [ $hang = 3 ]
