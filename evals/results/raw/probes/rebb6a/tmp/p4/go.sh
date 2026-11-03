cd /work/tmp/p4; J=/work/tmp/bin/just
t() { echo "=== $*"; "$@" 2>&1 | head -${H:-12}; }
t $J -f circ
t $J -f circ2
t $J -f circ3
t $J -f dep1 bar
t $J -f dep2 bar
t $J -f dep3 qux
t $J -f dep3 qq
for n in 20 22 24; do s=$(date +%s.%N); timeout 120 $J -f diamond$n a0 >/dev/null 2>&1; rc=$?; e=$(date +%s.%N); echo "n=$n rc=$rc t=$(echo "$e - $s" | python3 -c 'print(round(eval(input()),2))')"; done
