export TMPDIR=/tmp TSAN_OPTIONS="halt_on_error=0"
B=/target/tsan/aarch64-unknown-linux-gnu/debug/probe
cd /work/probe
for n in diff_send_race opts_race; do echo "=== $n"; timeout 200 $B $n > /tmp/t.$n 2>&1; python3 /work/tmp/sym.py $B /tmp/t.$n | head -46; done
