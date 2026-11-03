cd /work/tmp/p6; J=/work/tmp/bin/just; ulimit -c 0
t() { echo "=== $*"; "$@" 2>&1 | head -${H:-10} | cut -c1-180; }
for f in ic1 ic2 ic3 ic4 ic5 ic6 env1 env2\ foo env2\ bar env2\ baz env3 wd1\ foo wd1\ bar fs1 sub1; do t $J -f $f; done
echo y | $J -f lz3 2>&1 | head; 
t $J --timestamp --timestamp-format '%Q' -f sub1
$J -f bang2 foo; echo rc=$?; $J -f bang2 bar; echo rc=$?; $J -f sig foo 2>&1 | tail -2; $J -f sig foo >/dev/null 2>&1; echo rc=$?; $J -f sig bar 2>&1 | tail -2; $J -f sig bar >/dev/null 2>&1; echo rc=$?
for k in con and elif chain var; do for n in 5000 20000 100000; do s=$(date +%s.%N); timeout 60 $J -f $k$n --dump >/dev/null 2>/tmp/e; rc=$?; e=$(date +%s.%N); echo "$k$n rc=$rc $(python3 -c "print(round($e-$s,2))") $(head -c 120 /tmp/e | tr '\n' ' ')"; done; done
