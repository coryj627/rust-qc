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
set unstable

x := 'bar'
foo() := x

a:
  @echo {{x}} {{foo()}}
J
actual=$("$JUST" x=baz a 2>&1); rc=$?
echo "just x=baz a -> exit $rc, output: $actual"
echo "expected: baz baz"
if [ "$rc" = 0 ] && [ "$actual" = "baz bar" ]; then
  echo "DEMONSTRATED: override visible to the recipe, not to the function body"
  exit 0
fi
exit 1
