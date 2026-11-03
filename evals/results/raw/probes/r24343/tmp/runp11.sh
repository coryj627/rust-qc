export RUST_BACKTRACE=0; cd /work/tmp/probe; export CARGO_TARGET_DIR=/target/probe2
cargo build --offline --release --bin hog 2>&1 | tail -1
cd /work/tmp; for f in hog-0 hog-1 hog-2; do echo == $f; timeout 40 /target/probe2/release/hog $f.bin 2>&1 | tr -d '\0' | cut -c1-170 | tail -8; done
