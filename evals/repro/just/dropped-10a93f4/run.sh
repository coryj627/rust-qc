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
x := 'parent'

mod sub
J
cat > sub.just <<'J'
a := x
x := 'sub'

foo:
  @echo {{ a }} {{ x }}
J
actual=$("$JUST" sub::foo 2>&1); rc=$?
echo "just sub::foo -> exit $rc, output: $actual"
echo "expected: sub sub"
if [ "$rc" = 0 ] && [ "$actual" != "sub sub" ]; then
  echo "DEMONSTRATED: submodule assignment resolved to parent variable"
  exit 0
fi
exit 1
