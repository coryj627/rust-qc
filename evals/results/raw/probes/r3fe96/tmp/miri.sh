#!/bin/sh
cd /work/probe
export CARGO_TARGET_DIR=/target/miri
for t in deque_mc_uninit_read deque_mc_full_oob indexmap_retain_uaf spsc_peek_race sll_mut_then_pop_nonhead sll_mut_then_pop_head_single vec_intoiter_panic deque_clear_panic; do
  echo "=== $t"
  RUST_BACKTRACE=0 MIRIFLAGS="-Zmiri-disable-isolation" timeout 200 cargo +nightly-2026-10-01 miri test --offline --release --test p -- --exact $t --nocapture 2>&1 | grep -E "^(error|test |  *--> |thread)|Undefined Behavior|Data race|get\(4\)|len " | cut -c1-220 | head -10
done
