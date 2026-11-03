#!/bin/sh
cd /work/tmp/probe
T=+nightly-2026-10-01
filter() { grep -E "^error|Undefined Behavior|data race|Data race|boom|panicked|^[a-z_A-Z].*[0-9]$|-->|^test|free|dangling|out-of-bounds|Some|None|done|got |peek |\[" | grep -v "RUST_BACKTRACE" | cut -c1-200 | head -${1:-8}; }
for n in deque_full deque_mid_box sll_box sll_single into_iter_panic deque_clear hist_clear hist_write; do
  echo "--- miri $n"
  timeout 240 cargo $T miri run -q -- $n </dev/null 2>&1 | filter
done
for n in peek ready_c ready_p ctl_dequeue; do
  echo "--- miri many-seeds $n"
  MIRIFLAGS="-Zmiri-many-seeds=0..16" timeout 240 cargo $T miri run -q -- $n </dev/null 2>&1 | filter 8
done
for n in im_clear im_retain im_trunc; do
  echo "--- miri release SB $n"
  timeout 240 cargo $T miri run -q --release -- $n </dev/null 2>&1 | filter 12
  echo "--- miri release no-SB $n"
  MIRIFLAGS="-Zmiri-disable-stacked-borrows" timeout 240 cargo $T miri run -q --release -- $n </dev/null 2>&1 | filter 12
done
echo "--- retain SB"; timeout 240 cargo $T miri run -q -- retain_sb </dev/null 2>&1 | filter
echo "--- retain TB"; MIRIFLAGS=-Zmiri-tree-borrows timeout 240 cargo $T miri run -q -- retain_sb </dev/null 2>&1 | filter
echo "--- str_insert default"; timeout 240 cargo $T miri run -q -- str_insert </dev/null 2>&1 | filter
echo "--- str_insert recursive"; MIRIFLAGS=-Zmiri-recursive-validation timeout 240 cargo $T miri run -q -- str_insert </dev/null 2>&1 | filter
echo "--- pool native"; cargo build --offline -q 2>/dev/null; timeout 30 /target/debug/probe pool
echo "--- pool miri portable-atomic-cs"; MIRIFLAGS="-Zmiri-many-seeds=0..8" timeout 400 cargo $T miri run -q -- pool </dev/null 2>&1 | filter 10
