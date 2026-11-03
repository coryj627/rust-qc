#!/bin/sh
# Exit 0 when a one-entry table is shown to expand linearly with its integer key.
cd /work/repro/mlua-1 || exit 2
cargo build --release --offline 2>&1 | tail -1
B=/target/release/repro-mlua-1
fail=0
for m in ser de; do
  for k in 100000 1000000 4000000 16000000; do
    line=$($B $m $k) || fail=1
    echo "$line"
    len=$(echo "$line" | sed -n 's/.*output_len=\([0-9]*\).*/\1/p')
    if [ $m = ser ]; then want=$((5 * k - 1)); else want=$k; fi
    [ "$len" = "$want" ] || { echo "unexpected output_len $len, wanted $want"; fail=1; }
    rss=$(echo "$line" | sed -n 's/.*peak_rss_growth_kb=\([0-9]*\).*/\1/p')
    if [ $m = de ] && [ $k = 16000000 ] && [ "$rss" -lt 300000 ]; then echo "expected large RSS growth"; fail=1; fi
  done
done
[ $fail = 0 ] && echo "DEMONSTRATED: 1-entry table, output and memory grow linearly with the key"
exit $fail
