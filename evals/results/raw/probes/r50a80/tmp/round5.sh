cd /work/probe
export TMPDIR=/tmp
cargo build --offline --bin probe 2>&1 | grep -E "^error|^ +--> src|Finished" | head
sh run_all.sh /target/debug/probe pathspec_diff_dangling reflog_after_repo packwriter_double packbuilder_reuse rebase_after_finish
export ASAN_OPTIONS=detect_leaks=0
(export CFLAGS="-fsanitize=address -fno-omit-frame-pointer" CC=clang RUSTFLAGS="-Zsanitizer=address"
cargo +nightly-2026-10-01 build --offline --bin probe --target aarch64-unknown-linux-gnu --target-dir /target/asan 2>&1 | grep -E "^error|Finished|could not"
B=/target/asan/aarch64-unknown-linux-gnu/debug/probe
for n in pathspec_diff_dangling reflog_after_repo packwriter_double packbuilder_reuse rebase_after_finish; do
  echo "=== asan $n"; $B $n 2>&1 | grep -E "ERROR: AddressSanitizer|^READ of|^WRITE of|^[a-z0-9]+ |^\"" | cut -c1-150 | head -10
done)
(export CFLAGS="-fsanitize=thread -fno-omit-frame-pointer -g" CC=clang RUSTFLAGS="-Zsanitizer=thread"
cargo +nightly-2026-10-01 build --offline --bin probe --target aarch64-unknown-linux-gnu --target-dir /target/tsan 2>&1 | grep -E "^error|Finished|could not"
B=/target/tsan/aarch64-unknown-linux-gnu/debug/probe
export TSAN_OPTIONS="halt_on_error=0 history_size=4"
for n in diff_send_race opts_race; do
  echo "=== tsan $n"; timeout 240 $B $n > /tmp/t.$n 2>&1; grep -c "WARNING: ThreadSanitizer" /tmp/t.$n; grep -E "WARNING: ThreadSanitizer|SUMMARY" /tmp/t.$n | cut -c1-160 | sort | uniq -c | sort -rn | head -14
done)
