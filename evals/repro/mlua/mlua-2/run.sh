#!/bin/sh
# Exit 0 when a script-built nested table aborts the process through serde, in both directions.
cd /work/repro/mlua-2 || exit 2
cargo build --release --offline 2>&1 | tail -1
B=/target/release/repro-mlua-2
fail=0
for m in ser de; do
  $B $m 1000 || { echo "shallow run failed"; fail=1; }
  $B $m 100000 > /tmp/o 2>&1; code=$?
  cat /tmp/o
  echo "mode=$m depth=100000 exit=$code"
  [ $code = 134 ] && grep -q "has overflowed its stack" /tmp/o || fail=1
done
[ $fail = 0 ] && echo "DEMONSTRATED: stack overflow abort in serialize and deserialize"
exit $fail
