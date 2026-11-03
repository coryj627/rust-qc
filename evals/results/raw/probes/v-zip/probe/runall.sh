#!/bin/sh
# usage: runall.sh <bin> ; runs each test in its own process
B=$1
for t in "symlink_abort" "symlink_cap" "legacy 1 0" "legacy 1 1" "legacy 2 1" "legacy 6 1" "legacy 6 0" "shrink_stack 20000" "shrink_stack 200000" "shrink_real" "shrink8191 8190" "shrink8191 8191" "prefix" "aes_trunc" "aes_empty" "aes_under" "aes_readable" "aes_options" "aes_method" "z64_append" "z64_declared" "deep_after_append" "implode" "raw_crypto" "raw_meta" "large" "setuid" "mode0" "dirmode" "filemode_late" "dosro" "stream_meta" "stream_ds" "stream_empty" "stream_dd" "stream_crypto" "stream_aes" "stream_nodesc" "stream_abort" "sym_out" "sym_follow" "longname" "longcomment" "append_meta" "extra" "ioerr" "size_enforce" "fixed_crypto" "overlap_ovf" "seek_ovf" "seek_enc" "read_exact" "implode29" "rootdir_assert" "rootdir_dot" "lzma_skip" "symlink_path" "dir_dot" "merge_dup" "extra_poison" "levels" "crypto_date" "z64comment" "count" "trunc_comment" "zero_eocd" "backslash" "xz_empty" "by_path" "decomp_size" "unicode_append" "vacuous" "drop" "ppmd" "xz_index"; do
  echo "=== $t"
  timeout 120 $B $t > /work/tmp/one.out 2>&1
  rc=$?
  cut -c1-400 /work/tmp/one.out | grep -v "RUST_BACKTRACE"
  echo "--- exit $rc"
done
