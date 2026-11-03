cd /work/probe; export CARGO_TARGET_DIR=/target/probe
timeout 60 cargo test --offline --release --test soundness deque_overflow_canary -- --nocapture 2>&1 | grep -E "canary|0x|result|panicked" | cut -c1-160 | head -12
for t in indexmap_clear_panic_stale indexmap_truncate_panic_stale sorted_list_find_mut_modify_pop deque_contig_full_then_push_oob deque_contig_middle_branch_len; do
 echo "== miri $t"; MIRIFLAGS="-Zmiri-tree-borrows" timeout 90 cargo +nightly-2026-10-01 miri test --offline --release --test soundness $t 2>&1 | grep -E "^error: Undefined|^error: |test result|--> /work/crate" | cut -c1-200 | head -3
done
