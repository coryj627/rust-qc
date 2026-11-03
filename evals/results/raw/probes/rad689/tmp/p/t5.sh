J=/target/debug/just
mk() { d=$(mktemp -d -p /work/tmp/p); cd $d; cat > justfile; }
echo "--- 1 fmt private mod etc"
mk <<'J'
[private]
mod a

[private]
alias z := r

[doc('adoc')]
[group('g')]
mod b

[doc]
mod c

set allow-duplicate-recipes := false

eager v := 'x'

r: # keep me
  @echo hi

[doc('d')]
s:
  echo 'body' # stays
J
touch a.just b.just c.just
$J --list; $J --dump --unstable; cp justfile o; $J --fmt 2>&1; echo ---; cat justfile; echo ---; $J --list
$J -f o --json > j1; $J --json > j2; python3 -c "
import json;a=json.load(open('j1'));b=json.load(open('j2'))
def d(x,y,p=''):
  if isinstance(x,dict):
    for k in x:
      d(x[k],y.get(k),p+'/'+k)
  elif x!=y: print(p,x,y)
d(a,b)"
echo "--- 2 is_dependency in dep args"
mk <<'J'
[confirm(is_dependency())]
top: (dep is_dependency())
  @echo top
dep v:
  @echo dep={{v}}
J
echo y | $J top 2>&1
echo "--- 3 line number multi-line interp"
mk <<'J'
a:
  @echo {{ 'a' +
    'b' }}
  false
J
$J a 2>&1
echo "--- 4 script with path interp"
d=$(mktemp -d -p /work/tmp/p); cd $d; printf '#!/bin/sh\necho script=$(basename "$1")\n' > pwsh; chmod +x pwsh
cat > justfile <<J
[script('$d/pwsh')]
a:
  x
[script('pwsh')]
b:
  x
J
PATH=$d:$PATH $J a 2>&1; PATH=$d:$PATH $J b 2>&1
echo "--- 5 caret unicode"
mk <<'J'
x := "éééééééé" + y
J
$J 2>&1
printf 'x := "日本語" + y\na:\n' > justfile; $J 2>&1
echo "--- 6 guards doc"; grep -n -i "guard" /work/crate/README.md | head -20
echo "--- 7 export/unexport in module"
mk <<'J'
export A := 'a'
unexport B
mod m
foo:
  @echo A=${A:-} B=${B:-unset}
J
printf 'bar:\n  @echo A=${A:-unset} B=${B:-unset}\n' > m.just
B=fromenv $J foo; B=fromenv $J m bar
echo "--- 8 fmt with shebang interp and string kinds"
mk <<'J'
a := """
  x
"""
b := x"$HOME/foo"
c := f"{{a}}\n"
d := f"""
  {{a}}
"""
e := a + (b / c) && d
foo x=(a + b) y=`echo 1` z=f"{{a}}": (bar a + b) 
  echo
bar q:
J
$J --dump --unstable > d1 2>&1; $J --unstable -f d1 --dump > d2 2>&1; diff d1 d2 && echo fmt-stable; cat d1 | head -20
echo "--- 9 dep arg no-parens expression"
mk <<'J'
a: (b 'x' (c('y')))
b p q:
  @echo {{p}} {{q}}
c(x) := x
J
$J --unstable a 2>&1 | head -3
echo "--- 10 --set unknown in module, positional"
mk <<'J'
x := 'a'
a *args:
  @echo {{x}} {{args}}
J
$J a y=1 2>&1; $J y=1 a 2>&1 | head -2; $J --set x 2 a 2>&1; $J --set x 2 --set x 3 a; $J x=1 --set x 2 a; $J a 'x=1' 2>&1
echo "--- 11 evaluate quiet/private etc"
mk <<'J'
[private]
p := 'P'
_q := 'Q'
export r-s := 'R'
J
$J --evaluate; $J --evaluate p; echo; $J --variables; $J --evaluate --evaluate-format shell
echo "--- 12 newline in eval"
mk <<'J'
n := "a\nb$c"
J
$J --evaluate --evaluate-format shell
echo "--- 13 --one, --allow-missing"
mk <<'J'
a:
  @echo a
b:
  @echo b
J
$J --one a b 2>&1;$J --allow-missing nope 2>&1; echo rc=$?; $J --allow-missing --evaluate nope 2>&1; echo rc=$?;$J --allow-missing --show nope 2>&1; echo rc=$?; $J --allow-missing --list nope 2>&1; echo rc=$?;$J --allow-missing --usage nope 2>&1; echo rc=$?; $J --allow-missing nope::x 2>&1; echo rc=$?; $J --allow-missing a nope 2>&1; echo rc=$?;
echo "--- 14 fallback"
d=$(mktemp -d -p /work/tmp/p); mkdir -p $d/sub; cd $d/sub; printf 'top:\n  @echo top\n' > ../justfile; printf 'set fallback\nx:\n  @echo x\n' > justfile
$J top 2>&1; $J -v top 2>&1; $J nope 2>&1; $J --show top 2>&1 | head -2; $J --list 2>&1 ;$J x=1 x 2>&1
