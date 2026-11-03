#!/bin/sh
# Fix controls: the same reproducer sources built against each fix commit.
cd /work/repro
for id in mlua-1 mlua-2 mlua-3 mlua-5 mlua-6 mlua-7 mlua-8 mlua-9; do
  (cd $id/fix && cargo build --release --offline 2>&1 | grep -E '^error|Finished' -A8 | cut -c1-200)
done
B=/target/release
run() { echo "\$ $*"; "$@" 2>&1 | cut -c1-200; echo "status=$?"; }
{ for m in ser de; do for k in 9 10 1000 16000000; do $B/fix-mlua-1 $m $k; done; done; } > mlua-1/fix_output.txt 2>&1
{ for m in ser de; do for d in 100 128 129 100000; do $B/fix-mlua-2 $m $d; echo "mode=$m depth=$d exit=$?"; done; done; } > mlua-2/fix_output.txt 2>&1
{ for m in raw_set_nil raw_set_nan create_table_from_nil raw_set_nil_with_limit; do $B/fix-mlua-3 $m; echo "mode=$m exit=$?"; done; } > mlua-3/fix_output.txt 2>&1
{ for m in same_state_control foreign; do $B/fix-mlua-5 $m; echo "mode=$m exit=$?"; done; } > mlua-5/fix_output.txt 2>&1
{ for m in create_thread reset out_of_range; do $B/fix-mlua-6 $m; echo "mode=$m exit=$?"; done; } > mlua-6/fix_output.txt 2>&1
for id in mlua-7 mlua-8 mlua-9; do { $B/fix-$id; echo "exit=$?"; } > $id/fix_output.txt 2>&1; done
cd mlua-4/fix
RUSTFLAGS="-Zsanitizer=address" cargo +nightly-2026-10-01 build --release --target aarch64-unknown-linux-gnu --offline 2>&1 | grep -E '^error|Finished' -A8 | cut -c1-200
export ASAN_OPTIONS=detect_leaks=0
{ for m in method teardown; do /target/aarch64-unknown-linux-gnu/release/fix-mlua-4 $m 2>&1 | grep -vE '^ +#[0-9]+ ' | cut -c1-200 | head -14; echo "mode=$m exit=$?"; done; } > ../fix_output.txt 2>&1
