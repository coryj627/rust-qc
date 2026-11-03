#!/bin/sh
# Rebuilds offline and runs the reproducer. Exit 0 means the bug was demonstrated.
# REPRO_DIR is only overridden by repro/control.sh, which reruns this against the fix.
D=${REPRO_DIR:-/work/repro/git2-7}
BIN=git2_7
T=${CARGO_TARGET_DIR:-/target}
export RUST_BACKTRACE=0
cd "$D" || exit 2
OUT=$(mktemp)
build() { # profile flag
  if ! cargo build --offline --locked --quiet $2 2>"$OUT"; then
    grep -vE '^warning' "$OUT" | grep -E -A12 '^error' | head -40
    echo "BUILD FAILED ($1)"; exit 3
  fi
}
go() { # profile args...
  p=$1; shift
  { "$T/$p/$BIN" "$@"; } >"$OUT" 2>&1
  rc=$?
  cat "$OUT"
  echo "[$p $* exit=$rc]"
}
fail() { echo "NOT DEMONSTRATED: $1"; exit 1; }
build debug ""
build release --release

for op in add_all update_all remove_all; do
  for prof in debug release; do
    go $prof $op none
    [ $rc -eq 139 ] || fail "$prof $op with no pathspecs should die with SIGSEGV"
  done
done
go debug add_all emptystring
[ $rc -eq 139 ] || fail "add_all with [\"\"] should die with SIGSEGV"
echo "DEMONSTRATED: Index add_all/update_all/remove_all crash on empty pathspecs with a callback"
