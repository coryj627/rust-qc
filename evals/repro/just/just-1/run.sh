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

# justfile: a self-recursive user-defined function.
cat > justfile <<'J'
set unstable

foo() := foo()

bar:
  echo {{foo()}}
J
"$JUST" bar > out.txt 2>&1
rc=$?
echo "exit code: $rc"
cut -c1-200 out.txt | head -20
# SIGABRT from Rust's stack overflow handler shows up as 134 from sh.
if [ "$rc" -ge 128 ] && grep -q 'stack overflow\|has overflowed its stack' out.txt; then
  echo "DEMONSTRATED: process died on signal $((rc-128)) with a stack overflow"
  exit 0
fi
exit 1
