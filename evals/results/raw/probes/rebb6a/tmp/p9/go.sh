cd /work/tmp/p9; J=/work/tmp/bin/just; ulimit -c 0
t() { echo "=== $*"; "$@" 2>&1 | head -${H:-10} | cut -c1-200; }
t $J -f par
t $J -f lzm m::foo
t $J -f lzr n::foo
(chmod 755 path/d; PATH=$PWD/path:$PATH t $J -f wh)
t $J -f mdoc --list
t $J -f al2 --list
t $J -f al2 --list --alias-style separate
t $J -f al2 --show x
t $J -f dis other
t $J -f dis2 --list
(cd c1/c2/c3; rm -f ../../justfile; t $J --ceiling . r; t $J --ceiling .. r; t $J r; t $J --ceiling /work/tmp/p9/c1/c2 r; touch ../../justfile)
echo n | t $J -f win
t $J -f win </dev/null
t $J -f pos foo 1 2
t $J -f pos bar 'a b' c
t $J -f ovr x=z
t $J -f ovr --evaluate
t $J -f ovr2 q=1 foo
t $J -f ovr2 m::q=1 foo
t $J -f ovr2 nosuchmod::q=1 foo
t $J -f ovr2 m::x=1 --evaluate m::x
