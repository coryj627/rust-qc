cd /work/tmp/p11; J=/work/tmp/bin/just; ulimit -c 0; export PATH=/work/tmp/p11/bin:$PATH
t() { echo "=== $*"; "$@" 2>&1 | head -${H:-12} | cut -c1-260; }
t $J -f ps foo 1
t $J -f ps bar
t $J -f plain --choose
(cd "it's"; t $J --choose)
s=$(date +%s.%N); timeout 120 $J -f blow --list >/dev/null 2>&1; echo "blow --list rc=$? $(python3 -c "print(round($(date +%s.%N)-$s,1))")s"
for sub in --list --summary "recipe49999"; do s=$(date +%s.%N); timeout 120 $J -f big $sub >/dev/null 2>&1; echo "big $sub rc=$? $(python3 -c "print(round($(date +%s.%N)-$s,1))")s"; done
