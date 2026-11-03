#!/bin/sh
. /work/repro/lib.sh
prepare moka-3
# 1. Forced interleaving under Miri: this decides the exit code.
export MIRIFLAGS="-Zmiri-tree-borrows -Zmiri-ignore-leaks -Zmiri-permissive-provenance"
cargo +nightly-2026-10-01 miri run --offline --quiet --bin forced > /tmp/moka-3.out 2>&1
rc=$?
cut -c1-170 /tmp/moka-3.out
echo "miri exit code: $rc"
unset MIRIFLAGS
# 2. Same forced program natively, then plain thread stress: both are report only.
cargo build --offline --quiet --release --bins || exit 2
T=${CARGO_TARGET_DIR:-/target}/release
echo "--- native forced, 3 runs (report only) ---"
for i in 1 2 3; do
    "$T/forced" > /tmp/moka-3.native 2>&1
    echo "run $i exit=$? last: $(tail -1 /tmp/moka-3.native | cut -c1-140)"
    grep -m1 -E "panicked at|double free|free\(\)|corrupted" /tmp/moka-3.native | cut -c1-140
done
echo "--- native plain stress, ${STRESS_RUNS:-6} runs of ${STRESS_SECS:-5}s (report only) ---"
hits=0
i=0
while [ "$i" -lt "${STRESS_RUNS:-6}" ]; do
    i=$((i+1))
    "$T/stress" "${STRESS_SECS:-5}" > /tmp/moka-3.stress 2>&1
    e=$?
    echo "run $i exit=$e $(grep -m1 -E "panicked at|double free|free\(\)|corrupted|malloc" /tmp/moka-3.stress | cut -c1-140)"
    grep -A1 -m1 "panicked at" /tmp/moka-3.stress | tail -1 | cut -c1-140
    [ "$e" -ne 0 ] && hits=$((hits+1))
done
echo "plain stress runs with a crash or panic: $hits of $i"
if grep -q "Undefined Behavior" /tmp/moka-3.out && grep -qE "use-after-free|dangling|timer_wheel.rs" /tmp/moka-3.out; then
    echo "DEMONSTRATED: Miri reports UB in the timer wheel"
    exit 0
fi
echo "NOT DEMONSTRATED"
exit 1
