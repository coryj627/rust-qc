#!/bin/sh
# run each test in a separate process
cd /work/probe
PROF="$1"
cargo test --offline $PROF --test p --no-run 2>&1 | tail -1
BIN=$(cargo test --offline $PROF --test p --no-run --message-format=json 2>/dev/null | python3 -c "
import sys,json
for l in sys.stdin:
    try: d=json.loads(l)
    except: continue
    if d.get('executable') and d.get('target',{}).get('name')=='p': print(d['executable'])
")
for t in $($BIN --list 2>/dev/null | sed -n 's/: test$//p'); do
  echo "=== $t"
  RUST_BACKTRACE=0 timeout 120 $BIN --exact $t --nocapture --test-threads=1 2>&1 | grep -v -E '^(running|$|note: run with|test result|failures:|    |---- )' | cut -c1-200 | head -25
done
