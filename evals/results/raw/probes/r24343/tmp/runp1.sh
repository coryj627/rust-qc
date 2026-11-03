cd /work/tmp/probe; export CARGO_TARGET_DIR=/target/probe
cargo build --offline --bin p1 2>&1 | grep -E '^error|warning: unused|Finished' | head
for t in symhuge symbig sym modes unicode extras sfx seek aes stream; do echo "=== $t"; timeout 60 /target/probe/debug/p1 $t 2>&1 | grep -v 'note: run with' | cut -c1-400; echo "exit=$?/${PIPESTATUS}"; done
