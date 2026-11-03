#!/bin/bash
b=$1; shift
for n in "$@"; do echo "=== $n"; timeout 120 $b $n > /tmp/o.txt 2>&1; rc=$?; grep -v "RUST_BACKTRACE" /tmp/o.txt | cut -c1-260 | head -${LINES_MAX:-10}; echo "--> exit $rc"; done
