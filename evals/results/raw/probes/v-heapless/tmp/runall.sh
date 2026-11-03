#!/bin/sh
cd /work/tmp/probe
cargo build --offline -q 2>/dev/null; cargo build --offline -q --release 2>/dev/null
for prof in debug release; do
echo "=========== $prof"
for n in deque_full deque_full_pop deque_canary deque_mid deque_mid_box sll_nomut sll_u32 sll_box sll_single im_clear im_retain im_trunc into_iter_panic deque_clear hist_clear hist_write trunc trunc_search cap64k cap128k lm_from_iter bytes cstring im_into_iter lm_eq new_with sizes; do
  echo "--- $n"
  timeout 60 /target/$prof/probe $n </dev/null 2>&1 | grep -v "RUST_BACKTRACE\|^stack backtrace\|thread caused non-unwinding\|^ *[0-9]*:\|^ *at " | cut -c1-220 | head -14
  echo "[exit ${PIPESTATUS:-?}]"
done
done
