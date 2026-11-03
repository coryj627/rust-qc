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
J
ok=1
for args in "--show| " "--usage| " "--show|." "--usage|."; do
  flag=${args%%|*}; val=${args#*|}
  "$JUST" "$flag" "$val" > out.txt 2>&1
  rc=$?
  echo "--- just $flag '$val' -> exit $rc"; cut -c1-300 out.txt
  case "$val" in
    " ") if [ "$rc" = 101 ] && grep -q 'panicked at' out.txt && grep -q 'subcommand.rs' out.txt; then :; else ok=0; fi ;;
  esac
done
if [ "$ok" = 1 ]; then
  echo "DEMONSTRATED: whitespace-only path panics in resolve_path for --show and --usage"
  exit 0
fi
exit 1
