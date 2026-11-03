J=/target/debug/just
mk() { d=$(mktemp -d -p /work/tmp/p); cd $d; cat > justfile; }
echo "--- 1 nohup / ignored signals"
mk <<'J'
a:
  @kill -HUP $$; echo survived-hup
b:
  @kill -INT $$; echo survived-int
J
nohup $J a > out 2>&1; echo rc=$?; cat out
sh -c 'trap "" INT; '"$J"' b; echo rc=$?' 2>&1
sh -c 'trap "" INT; sh -c "kill -INT \$\$; echo plain-survived"' 2>&1
echo "--- 2 module doc multi-line dump"
mk <<'J'
[doc("a\nb")]
mod? x
J
$J --dump > d; cat d; $J -f d --dump 2>&1 | head -3
mk <<'J'
mod? x # first
J
$J --dump 2>&1
echo "--- 3 recipe with multiline doc"
mk <<'J'
[doc("a\nb")]
foo:
J
$J --dump; $J --list
echo "--- 4 dump of attributes with expressions"
mk <<'J'
x := 'p'
[confirm(x + 'y')]
foo:
  @echo hi
[metadata('a', 'b')]
[extension('.x')]
[script]
bar:
  true
J
$J --dump; $J --yes foo
echo "--- 5 parent variables in function, module function"
mk <<'J'
set unstable
g := 'root'
f() := g
mod m
J
printf 'set unstable\nh() := f()\nfoo:\n  @echo {{h()}}\n' > m.just
$J m foo 2>&1 | head -5
echo "--- 6 fn with fn arg using recipe param + backtick shell"
mk <<'J'
set unstable
id(x) := x
foo a:
  @echo {{id(a)}} {{id(id(a))}}
bar a=id('d'):
  @echo {{a}}
J
$J foo 1; $J bar
echo "--- 7 fn: dotenv & working dir"
mk <<'J'
set unstable
set dotenv-load
set working-directory := 'sub'
f() := `pwd`
g() := env('K')
foo:
  @echo {{f()}} {{g()}}
J
mkdir sub; echo K=V > .env; $J foo 2>&1
echo "--- 8 fn in settings"
mk <<'J'
set unstable
f() := 'sub'
set working-directory := f()
foo:
  @pwd
J
$J foo 2>&1 | head -4
echo "--- 9 eval with fn in lazy"
mk <<'J'
set unstable
set lazy
y := `echo ran-y >&2; echo Y`
z := `echo ran-z >&2; echo Z`
f() := y
foo:
  @echo {{f()}}
J
$J foo 2>&1
echo "--- 10 fn with assert/error"
mk <<'J'
set unstable
f(x) := if x == 'a' { 'ok' } else { error('bad ' + x) }
foo:
  @echo {{f('a')}} {{f('b')}}
J
$J foo 2>&1 | head -5
echo "--- 11 dump fn"
mk <<'J'
set unstable
f(x, y,) := x + y
z() := `echo`
J
$J --dump
echo "--- 12 assign named like constants in module / param"
mk <<'J'
set export
HEX2 := 'x'
foo:
  @echo ${HEX:-unset} ${HEX2:-unset} ${RED:-unset}
J
$J foo 2>&1
echo "--- 13 list alias with private target / unsorted"
mk <<'J'
alias x := _p
_p:
  @echo
[private]
q:
alias y := q
J
$J --list 2>&1; $J --summary; $J x 2>&1
echo "--- 14 quote in dump strings"
mk <<'J'
s := "a\tb\"c"
foo p="x\ny":
  @echo {{quote(p)}}
J
$J --dump; $J --evaluate; $J --evaluate --evaluate-format shell; $J foo
echo "--- 15 tempdir rel + script interpreter setting"
mk <<'J'
set script-interpreter := ['sh', '-e']
set tempdir := 'tt'
[script]
a:
  echo $0 | grep -c /tt/
  false
  echo not
J
mkdir tt; $J a 2>&1; ls tt
echo "--- 16 positional-arguments shebang + script"
mk <<'J'
set positional-arguments
a x y='d':
  #!/bin/sh
  echo $# $1 $2
[script('sh')]
b x:
  echo $# $1
c *r:
  @echo $# "$@"
J
$J a 1; $J b 1; $J c; $J c 'a b' c
