export RUST_BACKTRACE=0; cd /work/tmp/probe; export CARGO_TARGET_DIR=/target/probe2
cargo build --offline --bin p2 2>&1 | grep -E '^error|^  *--> src|Finished' | head
timeout 100 /target/probe2/debug/p2 aesforge 2>&1 | grep -v 'note: run with\|^$' | cut -c1-400
cd /work/crate; export CARGO_TARGET_DIR=/target/feat
for f in "" "aes-crypto" "deflate-flate2" "deflate-flate2-zlib-rs" "deflate-zopfli" "bzip2" "zstd" "lzma" "xz" "xz-static" "ppmd" "legacy-zip" "time" "chrono" "jiff-02" "nt-time" "deflate64"; do
  printf '%s: ' "[$f]"; cargo check --offline --locked --no-default-features --features "$f" 2>&1 | grep -E '^error(\[|:)|^warning: `zip`|Finished' | head -2 | tr '\n' ' ' | cut -c1-200; echo
done
