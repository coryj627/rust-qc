#!/bin/sh
# Exit 0 when the program reports wrong results (its own exit 42).
cd /work/repro/mlua-9 || exit 2
cargo build --release --offline 2>&1 | tail -1
/target/release/repro-mlua-9; code=$?
echo "program exit=$code"
[ $code = 42 ] && { echo "DEMONSTRATED: actual results differ from correct results"; exit 0; }
exit 1
