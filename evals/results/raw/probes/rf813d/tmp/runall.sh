#!/bin/sh
# usage: runall.sh <binary> name...
bin=$1; shift
for n in "$@"; do
  echo "=== $n"
  timeout 60 "$bin" "$n" </dev/null 2>&1 | grep -v "RUST_BACKTRACE\|^note: " | cut -c1-400 | head -${LINES_MAX:-16}
  # capture exit status with a second quiet run only if requested
done
