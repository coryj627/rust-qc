#!/bin/sh
. /work/repro/lib.sh
prepare moka-4
export MIRIFLAGS="-Zmiri-tree-borrows -Zmiri-ignore-leaks -Zmiri-permissive-provenance"
cargo +nightly-2026-10-01 miri run --offline --quiet > /tmp/moka-4.out 2>&1
rc=$?
cat /tmp/moka-4.out
echo "miri exit code: $rc"
if grep -q "Undefined Behavior" /tmp/moka-4.out && grep -q "deque.rs" /tmp/moka-4.out; then
    echo "DEMONSTRATED: Miri (Tree Borrows) reports UB with deque.rs in the backtrace"
    exit 0
fi
echo "NOT DEMONSTRATED"
exit 1
