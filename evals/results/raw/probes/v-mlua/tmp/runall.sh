#!/bin/sh
# usage: runall.sh bin case...
bin=$1; shift
for c in "$@"; do
  for prof in debug release; do
    echo "=== $c [$prof]"
    timeout ${T:-40} /work/tmp/target/$prof/$bin $c </dev/null 2>&1 | grep -v 'note: run with\|^stack backtrace' | cut -c1-260 | tail -${N:-10}
    echo "--- rc=$(timeout ${T:-40} /work/tmp/target/$prof/$bin $c </dev/null >/dev/null 2>&1; echo $?)"
  done
done
