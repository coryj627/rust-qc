#!/bin/sh
# usage: r.sh testfile name1 name2 ...
cd /work/probe
tf=$1; shift
CARGO_TARGET_DIR=/target/probe cargo test --offline --test $tf --no-run 2>&1 | grep -E '^error|Executable' | head -20
bin=$(ls -t /target/probe/debug/deps/$tf-* | grep -v '\.d$' | head -1)
for t in "$@"; do
  echo "=== $t"
  RUST_BACKTRACE=0 timeout 60 $bin "$t" --exact --nocapture --test-threads=1 2>&1 | grep -v 'server loop ended\|peer saw EOF\|(quiet)\|^running\|^$\|note: run with\|^test result' | cut -c1-300 | head -${LINES_MAX:-60}
done
