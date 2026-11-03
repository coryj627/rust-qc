#!/bin/sh
# Exit 0 when nil and NaN keys abort with no memory limit, while the protected path returns Err.
cd /work/repro/mlua-3 || exit 2
cargo build --release --offline 2>&1 | tail -1
B=/target/release/repro-mlua-3
fail=0
for m in raw_set_nil raw_set_nan create_table_from_nil; do
  $B $m > /tmp/o 2>&1; code=$?
  cat /tmp/o
  echo "mode=$m exit=$code"
  [ $code = 134 ] || fail=1
  grep -q "returned" /tmp/o && fail=1
done
$B raw_set_nil_with_limit > /tmp/o 2>&1; code=$?
cat /tmp/o
echo "mode=raw_set_nil_with_limit exit=$code (control: memory limit set, so the protected path runs)"
[ $code = 0 ] && grep -q "table index is nil" /tmp/o || fail=1
[ $fail = 0 ] && echo "DEMONSTRATED: safe raw_set and create_table_from abort the process"
exit $fail
