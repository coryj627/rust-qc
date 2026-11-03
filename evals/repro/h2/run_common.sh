#!/bin/sh
# Build and run one h2 reproducer offline. Args: <bug-id> [source-tree]
# source-tree defaults to the v0.4.12 baseline (/work/base). Pass a fix-commit
# tree to use it as the control. Each reproducer binary exits 0 ONLY when it has
# demonstrated the bug, non-zero otherwise.
set -e
id=$1
tree=${2:-/work/base}
src=/work/repro/$id
dst=/tmp/rp-$id-$(basename "$tree")
mkdir -p "$dst/src"
cp "$src"/src/*.rs "$dst/src/"
sed "s#/work/base#$tree#g" "$src/Cargo.toml" > "$dst/Cargo.toml"
cp "$tree/Cargo.lock" "$dst/Cargo.lock" 2>/dev/null || cp /work/base/Cargo.lock "$dst/Cargo.lock"
cd "$dst"
if ! cargo build --offline >build.log 2>&1; then echo "BUILD FAILED ($tree):"; tail -25 build.log; exit 3; fi
echo "== source tree: $tree ($(grep -m1 '^version' "$tree/Cargo.toml" | tr -d ' '))"
RUST_BACKTRACE=0 exec "${CARGO_TARGET_DIR:-/target}/debug/rp-$id"
