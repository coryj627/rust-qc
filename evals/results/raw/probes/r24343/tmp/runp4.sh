export RUST_BACKTRACE=0; cd /work/tmp/probe; export CARGO_TARGET_DIR=/target/probe2
cargo build --offline --bin p2 2>&1 | grep -E '^error|^  *--> src|Finished' | head
cargo build --offline --release --bin p2 2>&1 | grep -E '^error|Finished' | head
echo == debug more; timeout 120 /target/probe2/debug/p2 more 2>&1 | grep -v 'note: run with\|^$' | cut -c1-500
echo == release more; timeout 120 /target/probe2/release/p2 more 2>&1 | grep -v 'note: run with\|^$' | cut -c1-500
echo == release scale
timeout 200 /target/probe2/release/p2 eocdquad 4000 4000 2>&1 | cut -c1-200
timeout 200 /target/probe2/release/p2 eocdquad 8000 8000 2>&1 | cut -c1-200
timeout 200 /target/probe2/release/p2 eocdquad 30000 30000 2>&1 | cut -c1-200
timeout 100 /target/probe2/release/p2 overlap 200000 2>&1 | cut -c1-200
timeout 100 /target/probe2/release/p2 overlap 400000 2>&1 | cut -c1-200
timeout 200 /target/probe2/release/p2 big 2>&1 | cut -c1-200
timeout 100 /target/probe2/release/p2 shrinkrec 200000 2>&1 | tail -2 | cut -c1-200
