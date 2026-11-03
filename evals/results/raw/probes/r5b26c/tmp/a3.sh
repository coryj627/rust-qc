cd /work/probe2; export CARGO_TARGET_DIR=/target/probe2
echo "--- mpmc_large usize, 150s" > /work/tmp/aba_ctl.txt
timeout 200 env SECS=150 cargo test --offline --release --features heapless/mpmc_large --test aba -- --nocapture 2>&1 | grep -E "^sent|test result" | cut -c1-300 >> /work/tmp/aba_ctl.txt
for i in 1 2; do echo "--- u8 40s run $i" >> /work/tmp/aba_ctl.txt
timeout 100 env SECS=40 cargo test --offline --release --test aba -- --nocapture 2>&1 | grep -E "^sent|test result" | cut -c1-300 >> /work/tmp/aba_ctl.txt; done
echo finished >> /work/tmp/aba_ctl.txt
