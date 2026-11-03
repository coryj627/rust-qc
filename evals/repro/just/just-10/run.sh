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
  @echo hi
J
RUST_BACKTRACE=1 "$JUST" --timestamp --timestamp-format '%!' foo > out.txt 2>&1; rc=$?
echo "exit code: $rc"; head -4 out.txt | cut -c1-300; grep -E 'just::config::Config>::timestamp' out.txt
if [ "$rc" = 101 ] && grep -q 'just::config::Config>::timestamp' out.txt; then
  echo "DEMONSTRATED: panic on invalid --timestamp-format"
  exit 0
fi
exit 1
