#!/bin/sh
# usage: runall.sh binary names...
b=$1; shift
for n in "$@"; do echo "=== $n"; timeout 90 $b $n 2>&1 | grep -v "^stack backtrace\|RUST_BACKTRACE" | cut -c1-300 | head -${LINES_MAX:-12}; echo "--> exit ${PIPESTATUS:-?}"; done
