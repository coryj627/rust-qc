#!/bin/sh
# usage: runall.sh <binary> names...
B=$1; shift
for n in "$@"; do
  echo "=================== $n"
  timeout 30 "$B" "$n" 2>&1 | cut -c1-400 | head -30
  # exit status of the probe
  timeout 30 "$B" "$n" >/dev/null 2>&1; echo "[exit $?]"
done
