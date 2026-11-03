#!/bin/sh
# Rebuilds offline and runs the reproducer. Exit 0 means the bug was demonstrated.
# REPRO_DIR is only overridden by repro/control.sh, which reruns this against the fix.
D=${REPRO_DIR:-/work/repro/git2-5}
BIN=git2_5
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

go debug
[ $rc -eq 42 ] || fail "expected both checks to be wrong"
grep -q 'check1 WRONG' "$OUT" && grep -q 'check2 WRONG' "$OUT" || fail "missing WRONG lines"
echo "DEMONSTRATED: skip_reuc(true) fails conflicting merges and still writes REUC"
