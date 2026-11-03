#!/bin/sh
cd /work/crate
export CARGO_TARGET_DIR=/target/matrix
for f in "" aes-crypto bzip2 chrono deflate deflate-flate2 deflate-flate2-zlib-rs deflate-zopfli deflate64 jiff-02 lzma nt-time ppmd time xz zstd legacy-zip unreserved; do
  r=$(cargo check --locked --offline --lib --no-default-features --features "$f" 2>&1 | grep -E "^error|^warning: unused|^warning: .*(never|unused)" | sort | uniq -c | head -6)
  echo "[$f] ${r:-clean}"
done
