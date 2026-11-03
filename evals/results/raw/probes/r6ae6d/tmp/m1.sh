cd /work/tmp/probe
sh /work/tmp/runall.sh > /work/tmp/native.txt 2>&1
export MIRIFLAGS="-Zmiri-disable-isolation"
for spec in deque:second_branch_uninit_read deque:full_back_oob deque:clear_panic_double_drop sll:find_pop_after_mut_nonhead_box vecprobe:into_iter_drop_panic_double_drop imap:retain_panic_stale_indices imap:clear_panic_stale_indices races:spsc_consumer_peek_race; do
  t=${spec%%:*}; n=${spec##*:}
  printf '\n##### MIRI %s\n' "$spec"
  timeout 200 cargo +nightly-2026-10-01 miri test --offline --test $t -- --exact $n --nocapture 2>&1 | grep -E 'error|Undefined|panicked|drops=|test result|-->|canary|len ' | head -12
done
printf '\n##### release 64k\n'
timeout 200 cargo test --offline --release --test imap -- --nocapture --test-threads=1 pos_zero over_64k 2>&1 | grep -vE 'Compiling|^$' | tail -22
printf '\n##### compilecheck\n'
timeout 100 cargo test --offline --test compilecheck -- --nocapture 2>&1 | tail -6
