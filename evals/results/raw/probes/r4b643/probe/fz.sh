#!/bin/sh
B=/target/probe/debug/probe
for f in legacy/implode.zip lzma.zip deflate64.zip; do
  timeout 100 $B fuzz_mut $f 1500 > /tmp/f.txt 2>&1; echo "$f rc=$?"; tail -12 /tmp/f.txt | cut -c1-250
done
timeout 250 /target/probe/release/probe shrink_search 150 120 > /tmp/s.txt 2>&1; echo "search release rc=$?"; tail -12 /tmp/s.txt | cut -c1-700
timeout 200 $B shrink_search 40 100 > /tmp/s.txt 2>&1; echo "search debug rc=$?"; tail -12 /tmp/s.txt | cut -c1-700
