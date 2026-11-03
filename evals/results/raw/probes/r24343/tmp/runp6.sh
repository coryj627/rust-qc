export RUST_BACKTRACE=0; cd /work/tmp/probe; export CARGO_TARGET_DIR=/target/probe2
cargo build --offline --bin p3 2>&1 | grep -E '^error|^  *--> src|Finished' | head -20
timeout 120 /target/probe2/debug/p3 2>&1 | grep -v 'note: run with\|^$\|^ *[0-9]*: \|^ *at ' | cut -c1-420
