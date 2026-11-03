#!/bin/sh
export HOME=/work/tmp/home
B=/target/probe-asan/aarch64-unknown-linux-gnu/debug/probe
export ASAN_OPTIONS=detect_stack_use_after_return=1:detect_leaks=${LEAKS:-0}:abort_on_error=0
S=$(ls /usr/bin/llvm-symbolizer* /usr/lib/llvm-*/bin/llvm-symbolizer 2>/dev/null | head -1)
[ -n "$S" ] && export ASAN_SYMBOLIZER_PATH=$S
for n in "$@"; do
  echo "=== $n"
  timeout 120 $B "$n" </dev/null >/work/tmp/out.$$ 2>&1
  echo "exit=$?"
  awk '/ERROR: (Address|Leak)Sanitizer/{print; next} /^(READ|WRITE) of/{print; c=3; next} /^freed by/{print; c=4; next} /Direct leak/{print; c=5; next} c>0 && /^ *#[0-9]/{print; c--} /^SUMMARY/{print}' /work/tmp/out.$$ | sed -e 's#(BuildId.*##' -e 's#/work/crate/libgit2-sys/libgit2/##' -e 's# in core::ops::function.*##' | cut -c1-130 | head -${MAXL:-14}
done
