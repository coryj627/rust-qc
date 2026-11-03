#!/bin/sh
# usage: run_all.sh <binary> name...
bin=$1; shift
for n in "$@"; do
  echo "=== $n"
  "$bin" "$n" 2>&1 | grep -v "^note: \|^stack backtrace\|^ *[0-9]*: \|^ *at " | cut -c1-220 | head -14
done
