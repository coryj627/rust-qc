#!/bin/sh
bin=$1; shift
for t in "$@"; do
  echo "=== $t"
  timeout 200 $bin $t > /tmp/o.txt 2>&1 </dev/null
  rc=$?
  grep -v "^ *[0-9]*: \|^  *at " /tmp/o.txt | head -40 | cut -c1-300
  echo "--- exit=$rc"
done
