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
x := datetime('%!')
J
RUST_BACKTRACE=1 "$JUST" --evaluate x > out.txt 2>&1
rc=$?
echo "datetime exit code: $rc"; head -4 out.txt | cut -c1-300; grep -E 'just::function::datetime' out.txt
cat > justfile <<'J'
x := datetime_utc('%!')
J
RUST_BACKTRACE=1 "$JUST" --evaluate x > out2.txt 2>&1
rc2=$?
echo "datetime_utc exit code: $rc2"; head -4 out2.txt | cut -c1-300; grep -E 'just::function::datetime_utc' out2.txt
if [ "$rc" = 101 ] && grep -q 'just::function::datetime' out.txt && [ "$rc2" = 101 ] && grep -q 'just::function::datetime_utc' out2.txt; then
  echo "DEMONSTRATED: panic on invalid format string"
  exit 0
fi
exit 1
