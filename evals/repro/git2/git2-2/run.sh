#!/bin/sh
# Rebuilds offline and runs the reproducer. Exit 0 means the bug was demonstrated.
# REPRO_DIR is only overridden by repro/control.sh, which reruns this against the fix.
D=${REPRO_DIR:-/work/repro/git2-2}
BIN=git2_2
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
[ $rc -eq 134 ] || fail "debug build should abort"
grep -q 'unsafe precondition(s) violated: slice::from_raw_parts requires the pointer to be aligned and non-null' "$OUT" || fail "no precondition message"
grep -q 'src/remote.rs:387' "$OUT" || fail "abort not at Remote::list"
go release
[ $rc -eq 0 ] || fail "release should run on"
grep -q 'Some(list).is_none()=true' "$OUT" || fail "slice did not read back as null"
echo "DEMONSTRATED: Remote::list makes a null &[RemoteHead]"
