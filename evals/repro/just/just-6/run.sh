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
foo:
  echo {{ shell('touch shell-ran; echo hi') }}

bar:
  echo {{ `touch backtick-ran; echo hi` }}
J
"$JUST" --dry-run foo > out.txt 2>&1; rc=$?
echo "foo --dry-run exit $rc"; cut -c1-200 out.txt
"$JUST" --dry-run bar > out2.txt 2>&1; rc2=$?
echo "bar --dry-run exit $rc2"; cut -c1-200 out2.txt
ls -A
s=no; [ -e shell-ran ] && s=yes
b=no; [ -e backtick-ran ] && b=yes
echo "shell() command ran during --dry-run: $s (correct: no)"
echo "backtick command ran during --dry-run: $b (correct: no)"
if [ "$s" = yes ] && [ "$b" = no ]; then
  echo "DEMONSTRATED: --dry-run executed the shell() command"
  exit 0
fi
exit 1
