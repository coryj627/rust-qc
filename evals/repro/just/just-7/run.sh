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

printf 'bar:\n  @echo bar\n' > justfile
printf 'bar:\n' > .justfile
mkdir sub
printf 'set fallback\n\nfoo:\n' > sub/justfile
"$JUST" bar > direct.txt 2>&1; drc=$?
echo "in the parent directory: just bar -> exit $drc"; cut -c1-200 direct.txt
(cd sub && "$JUST" bar) > out.txt 2>&1; rc=$?
echo "in sub/ with set fallback: just bar -> exit $rc"; cut -c1-200 out.txt
if grep -qi 'multiple candidate justfiles' direct.txt \
   && ! grep -qi 'multiple candidate justfiles' out.txt \
   && grep -q 'does not contain recipe' out.txt; then
  echo "DEMONSTRATED: the ambiguity error is replaced by unknown-recipe"
  exit 0
fi
exit 1
