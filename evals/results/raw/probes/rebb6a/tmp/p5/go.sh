cd /work/tmp/p5; J=/work/tmp/bin/just
t() { echo "=== $*"; "$@" 2>&1 | head -${H:-14} | cut -c1-180; }
t $J -f m2 a::foo
t $J -f m2 b::foo
t $J -f m2 --list --list-submodules
t $J -f m3 a::show b::show
t $J -f m3 a::v=1 b::show
t $J -f m3 --set b::v 2 a::show
for f in fmtdoc fmtpriv; do cp $f $f.t; t $J --fmt -f $f.t; cat $f.t; t $J -f $f.t --list; done
t $J -f fmtpriv --list
t $J -f circmod
t $J -f circimp
t $J -f circ-self
