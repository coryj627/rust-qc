export RUST_BACKTRACE=0; cd /work/tmp/probe; export CARGO_TARGET_DIR=/target/probe2
cargo build --offline --bin p4 2>&1 | grep -E '^error|^  *--> src|Finished' | head -20
timeout 400 /target/probe2/debug/p4 2>&1 | grep -v 'note: run with\|^$' | cut -c1-360
