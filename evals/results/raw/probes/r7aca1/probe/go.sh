#!/bin/sh
# usage: go.sh name... ; runs each probe with a timeout and reports its exit status
for p in "$@"; do
  echo "=== $p"
  ( ulimit -v 2000000; timeout 30 /target/probe/debug/probe "$p" </dev/null 2>&1 | cut -c1-400 | tail -12; )
  timeout 30 /target/probe/debug/probe "$p" </dev/null >/dev/null 2>&1; echo "exit=$?"
done
