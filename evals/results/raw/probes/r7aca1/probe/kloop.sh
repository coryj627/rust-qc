#!/bin/sh
for k in $(seq 0 63); do
  K=$k timeout 20 "$1" mt_reuse 2>&1 </dev/null | grep -v "^victim\|^miss" | cut -c1-200
done
echo finished
