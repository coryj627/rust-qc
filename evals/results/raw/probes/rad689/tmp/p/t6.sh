J=/target/debug/just
mk() { d=$(mktemp -d -p /work/tmp/p); cd $d; cat > justfile; }
echo "--- 1 env attr vs export/dotenv/param default"
mk <<'J'
export FOO := 'var'
[env('FOO','attr')]
a:
  @echo line=$FOO
[env('BAR','attr')]
b p=`echo $BAR` q=shell('echo ${BAR:-unset}'):
  @echo p={{p}} q={{q}}
[env('FOO','attr')]
c:
  #!/bin/sh
  echo script=$FOO
J
$J a 2>&1; $J b 2>&1; $J c 2>&1
echo "--- 2 init with .justfile"
d=$(mktemp -d -p /work/tmp/p); cd $d; printf 'a:\n  @echo a\n' > .justfile; $J --init 2>&1; ls -a; $J a 2>&1 | head -4
echo "--- 3 show alias to module"
mk <<'J'
alias b := m::bar
mod m
J
printf 'bar:\n  @echo mod-bar\n' > m.just
$J --show b 2>&1; $J --list 2>&1; $J --usage b 2>&1
echo "--- 4 import + mod same file"
mk <<'J'
import 'x.just'
mod m 'x.just'
root:
  @echo root
J
printf 'v := "V"\nfoo:\n  @echo foo={{module_path()}}\n' > x.just
$J foo 2>&1;$J m foo 2>&1;$J root 2>&1; $J --summary
mk <<'J'
mod m 'x.just'
import 'x.just'
root:
  @echo root {{v}}
J
printf 'v := "V"\nfoo:\n  @echo foo={{module_path()}}\n' > x.just
$J foo 2>&1;$J m foo 2>&1;$J root 2>&1; $J --summary
echo "--- 5 unreadable ancestor"
d=$(mktemp -d -p /work/tmp/p); mkdir -p $d/a/b; chmod 311 $d/a; cd $d/a/b; id -u; $J --list 2>&1 | head -2; chmod 755 $d/a
echo "--- 6 editor"
mk <<'J'
a:
J
EDITOR='echo hi' $J --edit 2>&1 | head -3
echo "--- 7 arg count messages"
mk <<'J'
a x y='d':
b *r:
[arg('o',long)]
c o p:
d +v:
J
$J a 2>&1; $J a 1 2 3 2>&1; $J d 2>&1; $J c --o 1 2>&1; $J c 1 2>&1; echo ... ; $J --usage c
echo "--- 8 positional: colon arg first"
mk <<'J'
a x='':
  @echo x={{x}}
J
$J a b:c 2>&1; $J a 'b c' 2>&1;  $J 'a::' 2>&1; $J --show a:: 2>&1|head -2; $J --list a: 2>&1 | head -2
echo "--- 9 which/require"
mk <<'J'
set unstable
a:
  @echo [{{which('sh')}}] [{{which('nonexistent-x')}}] [{{which('/')}}] [{{which('.')}}] [{{which('..')}}]  [{{which('/bin')}}] 
b:
  @echo [{{which('')}}]
c:
  @echo [{{require('nonexistent-x')}}]
J
$J a 2>&1; $J b 2>&1 | head -2; $J c 2>&1 | head -2
echo "--- 10 functions misc"
mk <<'J'
a:
  @echo [{{parent_directory('/')}}] 
b:
  @echo [{{file_stem('a.b.c')}}] [{{without_extension('a')}}] [{{without_extension('/a/b.c')}}] [{{extension('a.b')}}] [{{file_name('a/b/')}}] [{{parent_directory('a')}}] [{{join('a','/b')}}] [{{clean('')}}] [{{clean('a/../..')}}] [{{clean('/..')}}]
c:
  @echo [{{capitalize('ßa')}}] [{{titlecase('fooBar baz')}}] [{{kebabcase('fooBar')}}] [{{choose('0','')}}] [{{choose('3','a')}}] [{{quote("a'b")}}] [{{replace_regex('abc','(?P<x>b)','[$x]')}}] [{{trim_end_match('aXX','X')}}]
d:
  @echo [{{choose('3','')}}]
e:
  @echo [{{semver_matches('1.2.3','>=1.0')}}] [{{semver_matches('1.2','>=1.0')}}]
f:
  @echo [{{encode_uri_component("a b/c?d=é~!*'()")}}] [{{sha256('')}}] [{{blake3('')}}]
g:
  @echo [{{style('error')}}x{{NORMAL}}] [{{absolute_path('../x')}}] [{{append('.x','a b')}}] [{{prepend('x','a  b')}}]
h:
  @echo [{{read('/nonexistent')}}]
i:
  @echo [{{source_file()}}] [{{source_directory()}}] [{{justfile()}}] [{{justfile_directory()}}] [{{module_file()}}] [{{module_directory()}}] [{{invocation_directory()}}]
J
for r in a b c d e f g h i; do $J $r 2>&1 | head -3; done
echo "--- 11 subdir invocation_directory etc in module/imports"
mkdir -p sub; printf 'import "../i.just"\nq:\n  @echo {{source_file()}} {{source_directory()}} {{module_directory()}}\n' > sub/mod.just; printf 'imp:\n  @echo {{source_file()}} {{source_directory()}} {{module_file()}}\n' > i.just; printf 'mod sub\n' > justfile; $J sub q 2>&1; $J sub imp 2>&1
echo "--- 12 tempdir, working dir nonexist"
mk <<'J'
set working-directory := 'nope'
a:
  @echo hi
b:
  #!/bin/sh
  echo hi
x := `echo hi`
J
$J a 2>&1|head -2; $J b 2>&1 | head -3
echo "--- 13 signal fails exit code"
mk <<'J'
a:
  kill -9 $$
b:
  -kill -9 $$
  @echo after
J
$J a 2>&1; echo rc=$?; $J b 2>&1; echo rc=$?
echo "--- 14 script exit w/ -"
mk <<'J'
a: b
  @echo a
b: && c
  @echo b
c:
  @exit 3
J
$J a 2>&1; echo rc=$?
echo "--- 15 duplicate dep runs, params"
mk <<'J'
a: (b '1') (b '1') (b '2') && (b '1') c
  @echo a
b x:
  @echo b{{x}}
c: (b '1')
J
$J a 2>&1
echo "--- 16 parallel"
mk <<'J'
[parallel]
a: b c d
b:
  @sleep 0.2; echo b
c: b
  @echo c
d: b
  @exit 2
J
$J a 2>&1; echo rc=$?
