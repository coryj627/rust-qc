#!/bin/sh
cd /work/tmp/probe
export MIRIFLAGS="-Zmiri-disable-isolation"
cargo +nightly-2026-10-01 miri setup --offline >/dev/null 2>&1
for t in "$@"; do
  echo "=================== $t"
  timeout 180 cargo +nightly-2026-10-01 miri test --offline --test ub -- --exact "$t" --nocapture 2>&1 | grep -E "^error|^test |Undefined Behavior|panicked|unwinding past|Data race|out-of-bounds|using uninit|dangling|free|^[0-9a-z{\[]" | grep -v '^error: test failed' | head -12
done
