J=/target/debug/just
mk() { d=$(mktemp -d -p /work/tmp/p); cd $d; cat > justfile; }
echo "--- 1 submodule var shadowed by parent, order"
mk <<'J'
y := 'parent'
mod m
J
cat > m.just <<'J'
x := y
y := 'child'
foo:
  @echo x={{x}} y={{y}}
J
$J m foo 2>&1
echo "--- 2 constant shadow"
mk <<'J'
A := HEX
HEX := 'mine'
foo:
  @echo A={{A}} HEX={{HEX}}
J
$J foo 2>&1
echo "--- 3 fn dynamic scoping"
mk <<'J'
set unstable
y := x
x := 'glob'
f(x) := y
g() := `echo ran >&2; echo bt`
v := `echo ran-v >&2; echo vv`
h() := v
foo:
  @echo {{f('param')}} {{x}} {{y}}
  @echo {{h()}} {{h()}} {{v}}
J
$J foo 2>&1
echo "--- 4 fn export env"
mk <<'J'
set unstable
export E := 'exported'
f() := `echo ${E:-unset}`
b := `echo ${E:-unset}`
foo:
  @echo fn={{f()}} body={{`echo ${E:-unset}`}}
J
$J foo 2>&1
echo "--- 4b fn param shadow"
mk <<'J'
set unstable
f(a, a) := a
foo:
  @echo {{f('1','2')}}
J
$J foo 2>&1
mk <<'J'
set unstable
f() := 'user'
uppercase(x) := 'mine'
foo:
  @echo {{uppercase('a')}}
J
$J foo 2>&1
echo "--- 5 fmt string dedent"
mk <<'J'
x := "a\nb"
p := '''

  a
'''
q := f'''

  a
'''
r := f'''
  {{x}}
  c
'''
foo:
  @printf '[%s]\n' "{{p}}"
  @printf '[%s]\n' "{{q}}"
  @printf '[%s]\n' "{{r}}"
J
$J foo 2>&1
echo "--- 6 long chains"
python3 - <<'P'
open('/work/tmp/p/long.just','w').write("x := " + " + ".join(["'a'"]*200000) + "\nfoo:\n  @echo hi\n")
open('/work/tmp/p/long2.just','w').write("x := " + "if 'a' == 'b' { 'c' } else "*100000 + "{ 'd' }\nfoo:\n  @echo hi\n")
P
$J -f /work/tmp/p/long.just foo 2>&1 | tail -2
$J -f /work/tmp/p/long2.just foo 2>&1 | tail -2
echo "--- 7 dep args options"
mk <<'J'
[arg("v", long)]
foo v="1" p="2":
  @echo v={{v}} p={{p}}
[arg("r", long)]
req r p:
  @echo r={{r}} p={{p}}
bar: (foo "x")
baz: (req "x")
J
$J bar 2>&1; $J baz 2>&1
echo "--- 8 assert const at list"
mk <<'J'
set lazy
x := assert("a" == "b", "boom")
foo:
  @echo hi
J
$J foo 2>&1; $J --list 2>&1; $J --dump 2>&1 | head -3
echo "--- 9 crlf escape"
printf 'x := "a\\\r\nb"\nfoo:\n  @echo {{x}}\n' > justfile; $J foo 2>&1 | head -5
echo "--- 10 ceiling relative"
d=$(mktemp -d -p /work/tmp/p); mkdir -p $d/a/b; cd $d/a/b; printf 'foo:\n  @echo FOUND\n' > $d/justfile; $J --ceiling . foo 2>&1; $J --ceiling $d/a foo 2>&1; $J --ceiling $d/a/b/ foo 2>&1
echo "--- 11 guard"
mk <<'J'
set guards
a: && c
  ?false
  @echo not-here
b:
  ?exit 2
c:
  @echo c
d:
  ?{{"true"}}
  #comment
  @echo ok
J
$J a 2>&1; echo rc=$?; $J b 2>&1; echo rc=$?; $J d 2>&1
echo "--- 12 continuation/sigils"
mk <<'J'
a:
  @echo a \
    b
  -@false
  @-false
  @@echo twice
b:
  echo {{""}}
  {{""}}@echo hi
  @{{"echo"}} x
J
$J a 2>&1; echo rc=$?; $J b 2>&1
