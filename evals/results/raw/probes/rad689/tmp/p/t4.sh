J=/target/debug/just
mk() { d=$(mktemp -d -p /work/tmp/p); cd $d; cat > justfile; }
echo "--- 11 fmt roundtrip"
mk <<'J'
# top comment

set shell := ['sh', '-cu']   # set comment
set dotenv-load
set unstable

export a := 'x' # c1
eager b := "y\n"
c := x'~/foo'
d := f'{{a}} and {{{{ {{b}}'
e := f"""
  foo {{a}}
"""
f1 := ```
  echo hi
```
g := if a == 'x' { 'p' } else if a =~ 'q' { 'r' } else { 's' }
h := a / b + c
i := / a
j := (a + b) / (c)
k := assert(a == 'x', "bad")
l := a && b || c
[private]
m := 'private'
_n := 'x'
u(p, q) := p + q
unexport FOO # uc

alias t := r1 # alias comment

# doc for mod
[group: 'g']
mod? mm 'mm.just' # mod comment

import? 'nothing.just' # imp comment

# recipe doc
[group('b'), group('a')]
[confirm('sure?')]
[env('A', "1\t2")]
[arg('z', long='zed', short='z', help="h", value='1', pattern='[0-9]')]
[private] # attr comment
@r1 $y='a' z=(a + 'b') *w: r2 (r3 'a' a) && r2 # dep comment
  -@echo {{ y }} {{{{ {{z + 'q'}} \
    continued
  # body comment

  echo last

r2: # rc
  @true

r3 p q: # rc2
# after
r4:

[script('python3', '-c')]
r5:
  print(1)

r6:
  #!/bin/sh

  echo {{ if a == 'x' { "y" } else { `echo z` } }}
J
cp justfile orig
$J --dump --unstable > dumped 2>&1; diff orig dumped
$J --fmt --unstable 2>&1; $J --fmt --check --unstable 2>&1 | head -5; 
$J --dump > d1 2>&1; $J --dump -f d1 >d2 2>&1; diff d1 d2 && echo idempotent
$J --justfile orig --json --unstable > j1 2>&1; $J --json --unstable > j2 2>&1; cmp j1 j2 && echo json-same
echo "--- 12 cases"
mk <<'J'
# c
[doc('x')]
a:

# not doc

[doc]
b:

[group('z')]
# mid comment
c:
J
$J --dump 2>&1
echo "--- 13 doc on mod w attr doc"
mk <<'J'
[doc('hello')]
mod? x
# d
mod? y
J
$J --dump 2>&1
echo "--- 14 fmt crlf"
printf 'a:\r\n  echo x\r\n' > justfile; $J --dump | od -c | head -3
echo "--- 15 option edge"
mk <<'J'
[arg('o', long, value='on')]
a o p='d':
  @echo o={{o}} p={{p}}
[arg('o', long)]
b o='def' *r:
  @echo o={{o}} r={{r}}
  @echo "$@"
[arg('o', short='é')]
c o='def':
  @echo o={{o}}
[positional-arguments, arg('o', long)]
d x o='def' y='ydef':
  @echo x={{x}} o={{o}} y={{y}} args="$@"
[arg('x', long)]
e x='' a='':
  @echo x={{x}} a={{a}}
J
$J a --o 2>&1; $J a p 2>&1 | head -3; $J b --o v 1 2 --o w 2>&1 | head -3; $J b 1 -- --o 2>&1; $J c -é v 2>&1; $J d X --o O Y 2>&1;  $J d X Y 2>&1; $J e --x=1 a e --x=2 a 2>&1;  $J e -- --x 2>&1; $J e --x -- 2>&1
$J --usage d 2>&1; $J --list 2>&1; $J --json | head -c 0
echo "--- 16 variadic dep with option"
mk <<'J'
[arg('o', long)]
a *r o='x':
  @echo
J
$J a 2>&1 | head -4
mk <<'J'
[arg('o', long)]
a o *r:
  @echo o={{o}} r={{r}}
b: (a '1' '2' '3')
J
$J b 2>&1 ; $J a --o=1 x y 2>&1
echo "--- 17 mod args"
mk <<'J'
mod m
a:
  @echo a
J
printf 'b x="d":\n  @echo b x={{x}}\nc:\n @echo c\n' > m.just
$J m b m c 2>&1; $J m::b 1 m::c 2>&1; $J m 2>&1; $J m::c:: 2>&1; $J m::  2>&1; $J 'm::c::' a 2>&1;  $J m::nope 2>&1;$J nope::c 2>&1; $J m nope 2>&1; $J a:: 2>&1;
