export RUST_BACKTRACE=0; cd /work/tmp/probe; export CARGO_TARGET_DIR=/target/probe2
cargo build --offline --release --bin fuzz 2>&1 | tail -1
cd /work/tmp; n=0; k=0
for f in $(grep -h 'rc=137' fuzz-*summary.log | awk '{print $2}'); do
  [ -f last-$f.bin ] || continue; k=$((k+1))
  /usr/bin/time -v true >/dev/null 2>&1
  timeout 40 /target/probe2/release/fuzz replay last-$f.bin >/dev/null 2>&1; rc=$?
  if [ $rc -ne 0 ]; then echo "last-$f.bin rc=$rc"; cp last-$f.bin hog-$n.bin; n=$((n+1)); [ $n -ge 3 ] && break; fi
done
echo "checked $k, hogs $n"
