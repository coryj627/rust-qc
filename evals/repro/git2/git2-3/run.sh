#!/bin/sh
# Rebuilds offline and runs the reproducer. Exit 0 means the bug was demonstrated.
# REPRO_DIR is only overridden by repro/control.sh, which reruns this against the fix.
D=${REPRO_DIR:-/work/repro/git2-3}
BIN=git2_3
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

for which in final orig; do
  go debug $which
  [ $rc -eq 134 ] || fail "debug $which should abort"
  grep -q 'null pointer dereference occurred' "$OUT" || fail "no null check message"
  grep -q 'src/signature.rs:73' "$OUT" || fail "not at Signature::name_bytes"
  go release $which
  [ $rc -eq 139 ] || fail "release $which should die with SIGSEGV"
done
echo "DEMONSTRATED: reading the signature of an uncommitted buffer-blame hunk faults"
