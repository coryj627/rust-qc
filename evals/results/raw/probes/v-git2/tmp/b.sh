cd /work/tmp/probe && cargo build 2>&1 | grep -E "^error|^ *--> src|Finished|could not" -A6 | cut -c1-160 | head -40
export TMPDIR=/tmp
LINES_MAX=6 /work/tmp/r.sh /target/debug/probe tag_symref clonenull_head clonenull_index clonenull_config clonenull_odb clonenull_status clonenull_revwalk clonenull_refs clonenull_find clonenull_is_bare clonenull_workdir clonenull_remotes clonenull_branches
export CC=clang CFLAGS="-fsanitize=address -fno-omit-frame-pointer -g" RUSTFLAGS="-Zsanitizer=address" CARGO_TARGET_DIR=/target/asan
cargo +nightly-2026-10-01 build --target aarch64-unknown-linux-gnu 2>&1 | tail -1 | cut -c1-200
export ASAN_OPTIONS=detect_leaks=0
echo "=== diff_race"; timeout 300 /target/asan/aarch64-unknown-linux-gnu/debug/probe diff_race > /tmp/o.txt 2>&1; echo "rc $?"; grep -E "ERROR: |survived|^READ|^WRITE" /tmp/o.txt | cut -c1-140 | head -5
unset CC CFLAGS RUSTFLAGS ASAN_OPTIONS; export CARGO_TARGET_DIR=/target
cd /work/crate
grep -n "git_cache__enabled\|git_object__strict\|strict_hash" libgit2-sys/libgit2/src/libgit2/settings.c | head
sed -n 15,30p libgit2-sys/libgit2/src/util/trace.c
grep -n "git_cache__enabled" libgit2-sys/libgit2/src/libgit2/*.c | head -5
