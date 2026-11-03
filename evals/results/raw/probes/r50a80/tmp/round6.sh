cd /work/probe
export TMPDIR=/tmp
(export CFLAGS="-fsanitize=thread -fno-omit-frame-pointer -g" CC=clang RUSTFLAGS="-Zsanitizer=thread -Cunsafe-allow-abi-mismatch=sanitizer"
cargo +nightly-2026-10-01 build --offline --bin probe --target aarch64-unknown-linux-gnu --target-dir /target/tsan 2>&1 | grep -E "^error|Finished|could not" | head -5
B=/target/tsan/aarch64-unknown-linux-gnu/debug/probe
export TSAN_OPTIONS="halt_on_error=0"
for n in diff_send_race opts_race; do
  echo "=== tsan $n"; timeout 200 $B $n > /tmp/t.$n 2>&1; echo "rc=$?"; grep -c "WARNING: ThreadSanitizer" /tmp/t.$n; grep -E "SUMMARY" /tmp/t.$n | cut -c1-170 | sort | uniq -c | sort -rn | head -14; tail -3 /tmp/t.$n | cut -c1-200
done)
cd /work/crate
cargo clippy --locked -p git2 --no-default-features --lib --message-format=short -- -W clippy::undocumented_unsafe_blocks -W clippy::missing_safety_doc -W missing_docs -W clippy::missing_errors_doc -W clippy::missing_panics_doc -W clippy::unwrap_used -W clippy::allow_attributes_without_reason -W clippy::map_err_ignore -W missing_debug_implementations 2>&1 | grep -E "^src/" | sed -E 's/^[^ ]+ (warning|error): //; s/`[^`]*`/X/g' | sort | uniq -c | sort -rn | head -14
