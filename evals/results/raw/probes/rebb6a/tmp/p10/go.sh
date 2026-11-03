cd /work/tmp/p10; J=/work/tmp/bin/just; ulimit -c 0
t() { echo "=== $*"; "$@" 2>&1 | head -${H:-12} | cut -c1-200; }
t $J -f col; t $J -f col2
cp fc fc.t; t $J --fmt -f fc.t; cat fc.t
t $J -f dr --dry-run foo; ls marker* ; rm -f marker*
t $J -f ex m::foo
t $J -f attrc; t $J -f attrn --dump
t $J -f dep a 1
t $J -f env4
t $J -f g
t $J -f ws
t $J -f quo
