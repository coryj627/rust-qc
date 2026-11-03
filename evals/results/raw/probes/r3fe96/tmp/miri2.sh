#!/bin/sh
cd /work/probe
export CARGO_TARGET_DIR=/target/miri
run() { # flags test
  echo "=== $2 [$1]"
  RUST_BACKTRACE=0 MIRIFLAGS="-Zmiri-disable-isolation $1" timeout 200 cargo +nightly-2026-10-01 miri test --offline --release --test p -- --exact $2 --nocapture 2>&1 | grep -E "^(error|test |  *--> |thread)|Undefined Behavior|Data race|get\(4\)|len |before|after" | cut -c1-220 | head -10
}
run "-Zmiri-disable-stacked-borrows" indexmap_retain_uaf
run "" deque_mc_box_double_free
run "" deque_mc_uninit_slot
run "" vec_retain_panic_sb
run "-Zmiri-tree-borrows" vec_retain_panic_sb
