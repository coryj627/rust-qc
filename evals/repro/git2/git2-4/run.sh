#!/bin/sh
# Rebuilds offline and runs the reproducer. Exit 0 means the bug was demonstrated.
# REPRO_DIR is only overridden by repro/control.sh, which reruns this against the fix.
D=${REPRO_DIR:-/work/repro/git2-4}
BIN=git2_4
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

# 1. Plain builds: the use-after-free shows up as a fault. Which fault depends on the
#    allocator state, so these are recorded but ASan below is the verdict.
go debug
go release
# 2. ASan on nightly, with libgit2's C instrumented too. Separate target dir.
AT=$T/asan
ASAN_TARGET=aarch64-unknown-linux-gnu
if ! CC=clang CFLAGS="-fsanitize=address -fno-omit-frame-pointer -g" \
     RUSTFLAGS="-Zsanitizer=address" CARGO_TARGET_DIR="$AT" \
     cargo +nightly-2026-10-01 build --offline --locked --quiet --target $ASAN_TARGET 2>"$OUT"; then
  grep -E -A12 '^error' "$OUT" | head -40; fail "ASan build failed"
fi
ASAN_OPTIONS=detect_leaks=0 "$AT/$ASAN_TARGET/debug/$BIN" >"$OUT" 2>&1
rc=$?
head -40 "$OUT"
echo "[asan exit=$rc]"
grep -q 'ERROR: AddressSanitizer: heap-use-after-free' "$OUT" || fail "ASan did not report heap-use-after-free"
# The image has no llvm-symbolizer, so name the frames with addr2line.
sed -n 's/^ *#\([0-9]*\) 0x[0-9a-f]* *(.*git2_4+\(0x[0-9a-f]*\)).*/\2/p' "$OUT" > "$OUT.addrs"
addr2line -f -e "$AT/$ASAN_TARGET/debug/$BIN" $(cat "$OUT.addrs") | grep -v '^/' | grep -v '^??' > "$OUT.names"
echo "symbolized frames, in report order:"; grep -nE 'git_refspec|git_remote_free|git_vector|get_refspec|str_bytes|git2_4' "$OUT.names" | head -20
sed -n 1p "$OUT.names" | grep -q 'git_refspec_string' || fail "faulting frame is not git_refspec_string"
grep -q 'git_remote_free' "$OUT.names" || fail "free stack does not name git_remote_free"
echo "DEMONSTRATED: heap-use-after-free reading a Refspec after its Remote dropped"
