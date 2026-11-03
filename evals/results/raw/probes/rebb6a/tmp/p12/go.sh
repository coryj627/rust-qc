cd /work/tmp/p12; J=/work/tmp/bin/just; R=/work/tmp/bin/just-rel; ulimit -c 0
t() { echo "=== $*"; "$@" 2>&1 | head -${H:-12} | cut -c1-240; }
t $J -f ga bar
t $R -f ga bar
t $J -f sd --show foo
t $J -f sd --show z
$J -f sd --json | python3 -c "import json,sys; d=json.load(sys.stdin); print(d['recipes']['foo']['dependencies'], d['aliases'])"
cd /work/tmp/p6
for k in con and elif chain var; do for n in 5000 20000 100000; do timeout 60 $R -f $k$n --dump >/dev/null 2>/tmp/e; echo "rel $k$n rc=$? $(head -c 60 /tmp/e | tr '\n' ' ')"; done; done
cd /work/tmp/p2; $R -f fn3 >/dev/null 2>/tmp/e; echo "rel fn3 rc=$? $(head -c 60 /tmp/e | tr '\n' ' ')"
