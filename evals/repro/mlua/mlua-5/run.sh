#!/bin/sh
# Exit 0 when set_metatable with a foreign table faults, and the same-state control does not.
cd /work/repro/mlua-5 || exit 2
cargo build --release --offline 2>&1 | tail -1
B=/target/release/repro-mlua-5
fail=0
$B same_state_control; code=$?
echo "mode=same_state_control exit=$code"
[ $code = 0 ] || fail=1
$B foreign > /tmp/o 2>&1; code=$?
cat /tmp/o
echo "mode=foreign exit=$code"
[ $code = 139 ] || fail=1
grep -q "set_metatable returned" /tmp/o && fail=1
echo "== ASan, shows the faulting address"
RUSTFLAGS="-Zsanitizer=address" cargo +nightly-2026-10-01 build --release --target aarch64-unknown-linux-gnu --offline 2>&1 | tail -1
export ASAN_OPTIONS=detect_leaks=0
for m in foreign foreign_0x42; do
  /target/aarch64-unknown-linux-gnu/release/repro-mlua-5 $m > /tmp/o 2>&1; code=$?
  echo "mode=$m exit=$code"
  grep -vE '^ +#[0-9]+ ' /tmp/o | cut -c1-170 | head -12
  grep -q "AddressSanitizer: SEGV" /tmp/o || fail=1
done
[ $fail = 0 ] && echo "DEMONSTRATED: memory fault in safe Table::set_metatable"
exit $fail
