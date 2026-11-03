cd /work/probe
export TMPDIR=/tmp
python3 /work/tmp/mkindex.py /tmp/idx
export CFLAGS="-fsanitize=address -fno-omit-frame-pointer" CC=clang
export RUSTFLAGS="-Zsanitizer=address"
cargo +nightly-2026-10-01 build --offline --target aarch64-unknown-linux-gnu --target-dir /target/asan 2>&1 | grep -E "^error|Finished|could not" | head
B=/target/asan/aarch64-unknown-linux-gnu/debug/probe
export ASAN_OPTIONS=detect_leaks=0:detect_stack_use_after_return=1
for n in refspec_dangling namespace_uaf workdir_uaf attr_uaf rebase_op_dangling revwalk_into_inner indexer_odb odb_double_finalize stash_moved_opts apply_moved_opts; do
  echo "=== $n"; $B $n 2>&1 | grep -E "ERROR: AddressSanitizer|^READ of|^WRITE of|#[0-6] .*(git2|probe|git_)|^[a-z]+ |^\"" | cut -c1-150 | head -12
done
echo "=== index"; $B index_flags_overread /tmp/idx 2>&1 | grep -E "ERROR: AddressSanitizer|^READ of|#[0-4] " | cut -c1-150 | head -8
