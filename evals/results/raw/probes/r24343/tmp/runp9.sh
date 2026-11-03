export RUST_BACKTRACE=0; cd /work/tmp
n=0
for f in $(grep -h 'rc=137' fuzz-*summary.log | awk '{print $2}' | shuf -n 25 --random-source=<(yes)); do
  [ -f last-$f.bin ] || continue
  o=$(timeout 40 /target/probe2/release/triage last-$f.bin 2>&1); rc=$?
  if [ $rc -ne 0 ]; then echo "last-$f.bin rc=$rc size=$(stat -c%s last-$f.bin): $(echo "$o" | tail -3 | tr -d '\0' | cut -c1-110 | tr '\n' '|')"; n=$((n+1)); [ $n -ge 4 ] && break; fi
done
echo checked
