J=/target/debug/just
mk() { d=$(mktemp -d -p /work/tmp/p); cd $d; cat > justfile; }
echo "--- 1 ignore-comments in script: undefined variable / fn"
mk <<'J'
set ignore-comments
a:
  #!/bin/sh
  # {{undefined_var}}
  echo a
b:
  #!/bin/sh
  # {{nofunc()}}
  echo b
c:
  #!/bin/sh
  # {{uppercase()}}
  echo c
d:
  echo d \
  # {{undefined_var}}
e:
  echo e \
  # {{nofunc()}}
J
for r in a b c d e; do $J $r 2>&1 | head -4; echo rc=$?; done
echo "--- 2 lazy + ignore-comments"
mk <<'J'
set lazy
set ignore-comments
v := 'V'
a:
  #!/bin/sh
  # {{v}}
  echo a
J
$J a 2>&1 | head -3
echo "--- 3 circular message"
mk <<'J'
a: b
b: c
c: b
J
$J 2>&1 | head -2
mk <<'J'
z: b
b: c
c: b
J
$J 2>&1 | head -2
mk <<'J'
a: b
b: c
c: d
d: c
J
$J 2>&1 | head -2
mk <<'J'
x := y
y := z
z := y
J
$J 2>&1 | head -2
echo "--- 4 ignored positional"
mk <<'J'
[group('g')]
a:
  @echo a
J
$J --groups extra 2>&1; $J x=1 --groups 2>&1 | head -2; $J --choose --chooser 'cat >/dev/null; echo a' extra 2>&1; $J --completions bash extra 2>&1 | head -1
echo "--- 5 param export to later param backtick"
mk <<'J'
a $x='1' y=`echo ${x:-unset}`:
  @echo y={{y}} x=$x body={{`echo ${x:-unset}`}}
J
$J a 2>&1
echo "--- 6 confirm under dry-run"
mk <<'J'
[confirm]
a:
  echo a
J
$J -n a </dev/null 2>&1; echo rc=$?
echo "--- 7 uuid var in fn"
mk <<'J'
set unstable
id := uuid()
f() := id
a:
  @echo {{id}}; echo {{f()}}
J
$J a
echo "--- 8 recipes with [private]/_ in dep list / default private"
mk <<'J'
_p:
  @echo private-default
b:
  @echo b
J
$J 2>&1; $J --list
echo "--- 9 subsequents with arguments and params"
mk <<'J'
a x: && (b x)
  @echo a {{x}}
b y:
  @echo b {{y}}
J
$J a 1
echo "--- 10 attribute env invalid names"
mk <<'J'
[env('A=B','x')]
a:
  @env | grep ^A
[env('','x')]
b:
  @echo b
J
$J a 2>&1 | head -5; $J b 2>&1 | head -4
echo "--- 11 null in strings / env"
mk <<'J'
x := "a\u{0}b"
a:
  @echo {{x}}
J
$J a 2>&1 | head -4
mk <<'J'
export x := "a\u{0}b"
a:
  @echo hi
J
$J a 2>&1 | head -4
mk <<'J'
[env('A', "a\u{0}b")]
a:
  @echo hi
b p:
  @echo hi
J
$J a 2>&1 | head -4; $J --set 'x' 'y' 2>&1 | head -2
echo "--- 12 shebang long / NUL recipe name ok, script ext"
mk <<'J'
a:
  #!/usr/bin/env -S sh -e
  echo env-s
b:
  #!/bin/sh -e
  echo with-arg
J
$J a 2>&1;$J b 2>&1
echo "--- 13 env-var edge"
mk <<'J'
a:
  @echo {{env('')}}
b:
  @echo {{env('A=B', 'def')}}
J
$J a 2>&1 | head -3; $J b 2>&1 | head -3
echo "--- 14 big numbers choose"
mk <<'J'
a:
  @echo {{choose('99999999999999999999','a')}}
b:
  @echo {{choose('-1','a')}}
J
$J a 2>&1 | head -2; $J b 2>&1 | head -2
