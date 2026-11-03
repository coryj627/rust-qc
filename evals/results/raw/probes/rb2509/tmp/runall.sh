#!/bin/sh
# usage: runall.sh BIN probe...
BIN=$1; shift
export HOME=/work/tmp/home; mkdir -p $HOME
for x in "$@"; do
  echo "=== $x"
  timeout 60 "$BIN" "$x" 2>&1 | cut -c1-400 | grep -v '^stack backtrace\|^note: run with\|^ *[0-9]*: \|^  *at ' | head -40
  echo "--- exit=$(timeout 60 "$BIN" "$x" >/dev/null 2>&1; echo $?)"
done
