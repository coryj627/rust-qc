#!/bin/sh
# Rebuilds offline, then runs the reproducer. Exit 0 = bug demonstrated, 1 = not demonstrated, 2 = build failed.
# Default tree is the 1.50.0 baseline. For the fix control run with TREE=/work/fixes/<short-sha>.
TREE=${TREE:-/work/base}
if [ "$TREE" != /work/base ]; then export CARGO_TARGET_DIR=/target/fix; fi
(cd "$TREE" && cargo build --locked --offline --bin just -q) || exit 2
JUST=${CARGO_TARGET_DIR:-/target}/debug/just
D=$(mktemp -d)
cd "$D" || exit 2
echo "tree: $TREE"
mkdir sub
printf "mod foo 'sub/foo.just'\n" > justfile
printf "mod bar '../justfile'\n" > sub/foo.just
/usr/bin/time -v "$JUST" --list > out.txt 2> err.txt; rc=$?
echo "cycle through '..': exit code $rc"
# The path in the message is thousands of characters long, so show its two ends.
head -1 err.txt | cut -c1-90
head -3 err.txt | grep -o 'File name too long.*\|is circular'
grep -E 'Elapsed|Maximum resident' err.txt
# Control: the same cycle written with an absolute path.
mkdir ctl ctl/sub
printf "mod foo 'sub/foo.just'\n" > ctl/justfile
printf "mod bar '%s'\n" "$D/ctl/justfile" > ctl/sub/foo.just
(cd ctl && "$JUST" --list) > ctl.txt 2>&1; crc=$?
echo "control, absolute path: exit code $crc"; head -1 ctl.txt | cut -c1-200
if [ "$rc" != 0 ] && grep -q 'File name too long' err.txt && ! grep -q 'circular' err.txt && grep -q 'circular' ctl.txt; then
  echo "DEMONSTRATED: the cycle through '..' is not detected; just recurses until the OS path limit"
  exit 0
fi
exit 1
