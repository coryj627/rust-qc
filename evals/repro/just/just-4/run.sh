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

mkdir root external
: > external/justfile
: > external/.justfile
printf "mod foo '%s'\n" "$D/external" > root/justfile
(cd root && "$JUST" --summary) > out.txt 2>&1
rc=$?
echo "exit code: $rc"; cut -c1-300 out.txt
if [ "$rc" = 101 ] && grep -q 'panicked at' out.txt && grep -q 'compiler.rs' out.txt; then
  echo "DEMONSTRATED: panic in compiler.rs building the ambiguity error"
  exit 0
fi
exit 1
