J=/target/debug/just
mk() { d=$(mktemp -d -p /work/tmp/p); cd $d; cat > justfile; }
echo "--- 1 dep positional binding to option/flag"
mk <<'J'
[arg('z', long, value='on')]
c z='off' p='p':
  @echo z={{z}} p={{p}}
e: (c 'bogus')
[arg('q', long, pattern='[0-9]+')]
f q='1' r='r':
  @echo q={{q}} r={{r}}
g: (f 'x')
J
$J e 2>&1 | head -3; $J g 2>&1 | head -3; $J c bogus 2>&1 | head -3
echo "--- 2 directory named justfile"
d=$(mktemp -d -p /work/tmp/p); mkdir -p $d/sub/justfile; printf 'a:\n  @echo root\n' > $d/justfile; cd $d/sub; $J a 2>&1 | head -3
mkdir -p $d/s2/Justfile; cd $d/s2; $J a 2>&1 | head -3
echo "--- 3 choose empty"
mk <<'J'
danger:
  @echo DEFAULT-RAN
other:
  @echo other
J
$J --choose --chooser 'cat >/dev/null' 2>&1; echo rc=$?
echo "--- 4 private recipe via choose, modules"
echo "--- 5 fmt keeps file on parse err; check rc"
mk <<'J'
a:
    echo
J
$J --fmt --check; echo rc=$?
echo "--- 6 settings in module vs unstable"
mk <<'J'
set unstable
mod m
J
printf 'f() := "x"\nfoo:\n  @echo {{f()}}\n' > m.just
$J m foo 2>&1 | head -2
echo "--- 7 import unstable, import set"
echo "--- 8 export param + --set + shell func"
mk <<'J'
v := 'orig'
s := shell('echo $1', v)
a:
  @echo {{s}}
J
$J v=new a 2>&1
echo "--- 9 trailing newline in --evaluate var / backticks crlf"
mk <<'J'
x := `printf 'a\n\n'`
y := `printf 'a\r\n'`
z := `printf ''`
J
$J --evaluate x | od -c | head -2; $J --evaluate y | od -c | head -1; 
echo "--- 10 non-utf8 backtick"
mk <<'J'
x := `printf '\377'`
a:
  @echo {{x}}
J
$J a 2>&1 | head -3
echo "--- 11 multiple -f / non-file"
$J -f /nonexistent a 2>&1 | head -2; $J -f / a 2>&1 | head -2; $J -f . a 2>&1 | head -2; $J -f .. a 2>&1 | head -2
echo "--- 12 search dir arg forms"
d=$(mktemp -d -p /work/tmp/p); mkdir -p $d/x/y; cd $d; printf 'a p="":\n  @echo a={{p}} pwd=$(basename "$PWD")\n' > x/justfile
$J x/ a 2>&1; $J x/a 2>&1; $J ./x/a b 2>&1;  cd x/y; $J ../a 2>&1; $J .. a 2>&1; $J . a 2>&1; $J a ../foo 2>&1; cd ..; $J y/../a 2>&1
echo "--- 13 --justfile & path-prefix"
cd $d; $J -f x/justfile x/a 2>&1 | head -2;  $J -f x/justfile -d . a 2>&1
echo "--- 14 alias with args, alias in module invoked"
mk <<'J'
alias c := m::d
mod m
J
printf 'alias e := d\nd x="1":\n  @echo d={{x}}\n' > m.just
$J c 2 2>&1; $J m e 3 2>&1; $J m::e 2>&1; $J --usage m::e 2>&1 | head -2; $J --show m e 2>&1 | head -3
echo "--- 15 recipe doc with non-attr in json; variables modules"
$J --variables; $J --groups; $J --summary
echo "--- 16 list module nesting + --list-submodules + private recipe-only"
mk <<'J'
mod m
J
printf '_only:\n' > m.just
$J --list --list-submodules 2>&1; $J --list m 2>&1; $J --list x 2>&1 | head -1
