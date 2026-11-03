cd /work/probe
BIN=$(ls -t /target/probe/debug/deps/soundness-* | grep -v '\.d$' | head -1)
for t in $($BIN --list 2>/dev/null | sed -n 's/: test$//p'); do
  echo "=== $t"
  timeout 20 $BIN --exact $t --test-threads=1 --nocapture 2>&1 | grep -vE "RUST_BACKTRACE|^running|^$|test result" | cut -c1-200 | tail -8
  echo "rc=${PIPESTATUS[0]}"
done
