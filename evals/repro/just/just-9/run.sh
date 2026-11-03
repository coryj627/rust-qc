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

cat > justfile <<'J'
a := HEX

HEX := 'foo'
J
actual=$("$JUST" --evaluate a 2>&1); rc=$?
echo "just --evaluate a -> exit $rc, output: $actual"
echo "expected: foo"
over=$("$JUST" --set HEX bar --evaluate a 2>&1); rc2=$?
echo "just --set HEX bar --evaluate a -> exit $rc2, output: $over"
echo "expected: bar"
all=$("$JUST" --evaluate HEX 2>&1)
echo "just --evaluate HEX -> $all"
if [ "$rc" = 0 ] && [ "$actual" = "0123456789abcdef" ]; then
  echo "DEMONSTRATED: reference resolved to the builtin constant, not the assignment"
  exit 0
fi
exit 1
