export RUST_BACKTRACE=0; cd /work/tmp/probe; export CARGO_TARGET_DIR=/target/probe
cargo build --offline --release --bin p2 --bin fuzz 2>&1 | tail -1
drive() { # profile base
  prof=$1; base=$2; end=$(( $(date +%s) + 280 )); s=$base
  while [ $(date +%s) -lt $end ]; do
    rem=$(( end - $(date +%s) )); s=$((s+1))
    ( timeout $((rem+20)) /target/probe/$prof/fuzz $s $rem ) > /work/tmp/fuzz-$prof-$s.log 2>&1
    rc=$?
    echo "seed $s rc=$rc" >> /work/tmp/fuzz-$prof-summary.log
    if [ $rc -ne 0 ]; then cp /work/tmp/last-$s.bin /work/tmp/abort-$prof-$s.bin; tail -3 /work/tmp/fuzz-$prof-$s.log >> /work/tmp/fuzz-$prof-summary.log; fi
  done
}
drive debug 5000 & drive release 6000 & drive release 7000 &
wait
