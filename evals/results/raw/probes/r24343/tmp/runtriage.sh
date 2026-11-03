export RUST_BACKTRACE=0; cd /work/tmp/probe; export CARGO_TARGET_DIR=/target/probe2
cargo build --offline --release --bin triage 2>&1 | tail -1
cd /work/tmp; n=0
for f in $(ls abort-*.bin | shuf -n 60 --random-source=<(yes)); do
  ( ulimit -v 1500000; timeout 30 /target/probe2/release/triage $f 2>&1 | tail -4 | cut -c1-140 | tr '\n' '|' ); echo
done | sed -E 's/^[^|]*\| *//' | sort | uniq -c | sort -rn | head -40
