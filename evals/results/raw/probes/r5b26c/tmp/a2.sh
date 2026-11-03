cd /work/probe2; export CARGO_TARGET_DIR=/target/probe2
timeout 260 env SECS=180 cargo test --offline --release --test aba -- --nocapture 2>&1 | grep -E "^sent|test result|panicked" | cut -c1-400 > /work/tmp/aba_spin.txt; echo "rc done" >> /work/tmp/aba_spin.txt
