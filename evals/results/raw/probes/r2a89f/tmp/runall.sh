#!/bin/sh
cd /work/tmp/probe
env | grep -i cargo_target
BIN=$(cargo test --locked -p quinn-proto --lib --no-run --message-format=short 2>&1 | tee /work/tmp/build.txt | grep -oE '/[^ )]*deps/quinn_proto-[0-9a-f]+|target/debug/deps/quinn_proto-[0-9a-f]+' | tail -1)
tail -5 /work/tmp/build.txt
echo BIN=$BIN
: > /work/tmp/out.txt
for t in $($BIN --list 2>/dev/null | grep ': test' | grep audit | sed 's/: test//'); do
  echo "=== $t" >> /work/tmp/out.txt
  timeout 40 $BIN --exact "$t" --nocapture --test-threads=1 2>&1 | grep -aE 'OBS|panicked at|^test result|left:|right:|^[a-zA-Z`].*expected|in-progress|connect' >> /work/tmp/out.txt
  echo "exit=$?" >> /work/tmp/out.txt
done
