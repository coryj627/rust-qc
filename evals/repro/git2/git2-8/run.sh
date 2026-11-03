#!/bin/sh
# Rebuilds offline and runs the reproducer. Exit 0 means the bug was demonstrated.
# REPRO_DIR is only overridden by repro/control.sh, which reruns this against the fix.
D=${REPRO_DIR:-/work/repro/git2-8}
BIN=git2_8
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

for prof in debug release; do
  go $prof
  [ $rc -eq 134 ] || fail "$prof should abort"
  grep -q 'left: -3' "$OUT" || fail "$prof: not the assert on GIT_ENOTFOUND"
  grep -q 'src/repo.rs:966' "$OUT" || fail "$prof: not at Repository::submodules"
  grep -q 'panic in a function that cannot unwind' "$OUT" || fail "$prof: no abort at the extern C boundary"
done
echo "DEMONSTRATED: Repository::submodules aborts the process on a lookup failure"
