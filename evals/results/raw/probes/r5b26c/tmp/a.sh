cd /work/probe2; export CARGO_TARGET_DIR=/target/probe2
timeout 200 env SECS=100 cargo test --offline --release --test aba -- --nocapture 2>&1 | grep -E "^sent|test result|panicked" | cut -c1-300 > /work/tmp/aba_u8.txt
echo "--- mpmc_large (usize positions)"  >> /work/tmp/aba_u8.txt
timeout 200 env SECS=100 cargo test --offline --release --features heapless/mpmc_large --test aba -- --nocapture 2>&1 | grep -E "^sent|test result|panicked|^error" | cut -c1-300 >> /work/tmp/aba_u8.txt
