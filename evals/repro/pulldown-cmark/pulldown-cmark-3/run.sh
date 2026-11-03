#!/bin/sh
# Rebuilds offline and runs the reproducer. Exit 0 means the bug was demonstrated.
# Set PCM_SRC=/work/fix/<dir>/pulldown-cmark to run the same harness against a fix commit.
set -u
ID=pulldown-cmark-3
D=/work/repro/$ID
SRC=${PCM_SRC:-/work/base/pulldown-cmark}
W=/tmp/repro-$ID
mkdir -p "$W"
cp -r "$D/src" "$D/Cargo.toml" "$W/"
cp /work/base/Cargo.lock "$W/Cargo.lock"
sed -i "s#/work/base/pulldown-cmark#$SRC#" "$W/Cargo.toml"
cd "$W" || exit 2
export CARGO_TARGET_DIR=/target/repro/$ID
echo "pulldown-cmark source: $SRC"
cargo build -q --offline || exit 2
cargo build -q --release --offline || exit 2
fail=1
for prof in debug release; do
  echo "--- profile $prof"
  out=$("$CARGO_TARGET_DIR/$prof/repro" 2>&1); rc=$?
  echo "$out"; echo "exit status: $rc"
  [ "$prof" = debug ] && [ $rc -ne 0 ] && echo "$out" | grep -q 'panicked at' && d=1
  [ "$prof" = release ] && [ $rc -ne 0 ] && echo "$out" | grep -q 'panicked at' && r=1
done
[ "${d:-0}" = 1 ] && [ "${r:-0}" = 1 ] && exit 0
exit 1

