cd /work/probe; export CARGO_TARGET_DIR=/target/probe
cargo test --offline --test misc --no-run 2>&1 | tail -1 | cut -c1-200
BIN=$(ls -t /target/probe/debug/deps/misc-* | grep -v '\.d$' | head -1)
for t in $($BIN --list 2>/dev/null | sed -n 's/: test$//p'); do
  echo "=== $t"; timeout 20 $BIN --exact $t --test-threads=1 --nocapture 2>&1 | grep -vE "RUST_BACKTRACE|^running|^$|test result|^failures" | cut -c1-200 | tail -6; echo "rc=${PIPESTATUS[0]}"
done
