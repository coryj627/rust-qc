cd /work/tmp/p8; J=/work/tmp/bin/just; ulimit -c 0
t() { echo "=== $*"; "$@" 2>&1 | head -${H:-10} | cut -c1-200; }
(cd a; t $J --init; ls -a; t $J --list)
(cd nest/x/y; t $J local; t $J --ceiling .. local; t $J --ceiling ../.. root; t $J --ceiling /work/tmp/p8/nest/x root; t $J --ceiling /work/tmp/p8/nest/x/y local; t $J root; t $J -v root)
for f in lexa lexb lexc mix; do t $J -f $f --dump; done
t $J -f noisy --dry-run
$J -f noisy 2>/dev/full; echo "rc-devfull=$?"
$J -f noisy big 2>&1 >/dev/null | true; echo "rc=${PIPESTATUS[0]}"
(exec 2> >(true); sleep 0.2; $J -f noisy big >/dev/null; echo "rcerr=$?" >&1)
$J -f q2 --list >/dev/full; echo "rc-list-full=$?"
t $J -f src1 top; t $J -f src1 ifoo; t $J -f src1 jfoo
t $J -f def1
t $J -f def1 --list
$J -f cmd1 --command sh -c 'echo A=$A; exit 3'; echo rc=$?
t $J -f q2 --quiet --time foo
t $J -f q2 --quiet --timestamp foo
t $J -f q2 --quiet --explain foo
t $J -f q2 --timestamp foo
cp crlf crlf.t; t $J --fmt -f crlf.t; od -c crlf.t | head -6; 
t $J -f mdir m::foo
t $J -f chs --choose --chooser 'cat > /dev/null'
EDITOR='echo edit-with-args -x' t $J -f chs --edit
