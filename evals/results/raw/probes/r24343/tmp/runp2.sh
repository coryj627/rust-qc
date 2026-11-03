cd /work/tmp/probe; export CARGO_TARGET_DIR=/target/probe
cargo build --offline --bin p2 2>&1 | grep -E '^error|^  *--> src|Finished' | head -20
for t in stream wstream readable append raw api legacy "shrinkrec 1000" "shrinkrec 200000" "eocdquad 2000 2000" "eocdquad 4000 4000" "overlap 20000" "overlap 40000"; do echo "=== $t"; timeout 120 /target/probe/debug/p2 $t 2>&1 | grep -v 'note: run with' | cut -c1-500; echo "exit=${PIPESTATUS[0]}"; done
