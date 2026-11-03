#!/bin/sh
# Rebuilds offline and runs the reproducer. Exit 0 means the bug was demonstrated.
# REPRO_DIR is only overridden by repro/control.sh, which reruns this against the fix.
D=${REPRO_DIR:-/work/repro/git2-6}
BIN=git2_6
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
[ $rc -eq 101 ] || fail "expected a Rust panic (exit 101)"
grep -q 'src/reference.rs:498' "$OUT" || fail "panic not at ReferenceNames::next"
grep -q 'Utf8Error' "$OUT" || fail "panic is not the from_utf8 unwrap"
echo "DEMONSTRATED: ReferenceNames panics on a non-UTF-8 ref name"
