J=/target/debug/just
mk() { d=$(mktemp -d -p /work/tmp/p); cd $d; cat > justfile; }
echo "--- 1 dep args options"
mk <<'J'
[arg("v", long)]
foo v="1" p="2":
  @echo v={{v}} p={{p}}
bar: (foo "x")
J
$J bar 2>&1
echo "--- 2 same file two modules"
mk <<'J'
mod a 'x.just'
mod b 'x.just'
J
cat > x.just <<'J'
v := 'def'
foo:
  @echo foo v={{v}} path={{module_path()}}
J
$J a foo 2>&1; $J b foo 2>&1;$J a foo b foo 2>&1; $J a::v=1 a foo 2>&1; $J b::v=1 b foo 2>&1;  $J --summary 2>&1
echo "--- 3 ambiguous module .. panic"
d=$(mktemp -d -p /work/tmp/p); mkdir -p $d/y $d/other; cd $d/y
printf "mod foo '../other'\n" > justfile; printf 'a:\n' > ../other/mod.just;  printf 'a:\n' > ../other/justfile
$J --list 2>&1 | head -5
echo "--- 4 mod with file of the same name"
mk <<'J'
mod? bin
a:
  @echo a
J
echo hi > bin; $J a 2>&1
echo "--- 5 datetime bad fmt"
mk <<'J'
x := datetime('%Q')
a:
  @echo {{x}}
J
$J a 2>&1 | head -5; 
mk <<'J'
a:
  @echo hi
J
$J --timestamp --timestamp-format '%Q' a 2>&1 | head -5
echo "--- 6 show blank"
$J --show ' ' 2>&1 | head -4; $J --usage ' ' 2>&1 | head -4; $J --show '' 2>&1 | head -4; $J --evaluate ' ' 2>&1 | head -4;$J --list ' ' 2>&1 | head -4
echo "--- 7 env bools"
JUST_QUIET=0 $J a 2>&1 | head -3; JUST_VERBOSE=1 $J a 2>&1 | head -3;JUST_VERBOSE=0 $J a 2>&1 | head -3; JUST_DRY_RUN=false $J a 2>&1 | head -3; JUST_DRY_RUN=1 $J a 2>&1 | head -3; JUST_UNSTABLE=0 $J --list 2>&1 | head -2
echo "--- 8 command wd / dry-run / shell() dry-run"
mkdir -p sub
cat > justfile <<'J'
set working-directory := 'sub'
x := shell('touch shell-ran; echo y')
a:
  @pwd
J
$J a; rm -f sub/shell-ran; ls sub; $J --command pwd 2>&1; rm -f sub/shell-ran; $J --dry-run a 2>&1; ls sub; rm -f sub/shell-ran;  $J --dry-run --command touch cmd-ran 2>&1; ls . sub
echo "--- 9 quiet script"
mk <<'J'
a:
  #!/bin/sh
  echo script-out
  echo script-err >&2
b:
  echo line-out
[script('sh')]
c:
  echo c-out
J
$J --quiet a 2>&1; $J --quiet b 2>&1; $J --quiet c 2>&1; echo "-ts"; $J --quiet --timestamp --timestamp-format=T b 2>&1
echo "--- 10 private module choose, alias list"
mk <<'J'
alias b := m::bar
[private]
mod p
mod m
bar:
  @echo local-bar
J
printf 'sec:\n  @echo secret\n' > p.just; printf 'bar:\n  @echo mod-bar\n' > m.just
$J --list 2>&1; $J --choose --chooser 'cat >/dev/null; echo sec; cat /dev/null' 2>&1 | head -3; $J --choose --chooser 'cat 1>&2; echo bar' 2>&1 | head
echo "--- 10b choose with module recipe"
$J --choose --chooser 'echo m bar' 2>&1
$J --choose --chooser 'cat >/dev/null; printf "m bar\nbar\n"' 2>&1
$J b 2>&1
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
@r1 $y='a' z=(a + 'b') +w: r2 (r3 'a' a) && r2 # dep comment
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
echo "--- 12 eval just fmt width"
mk <<'J'
abc := 'a"b'
x := 'y'
J
$J --evaluate; $J --variables
