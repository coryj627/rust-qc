#!/bin/bash
. /work/tmp/lib.sh
t "stdout panic"
python3 -c "
for i in range(20000): print(f'r{i}:')" > justfile
$J --list 2>err | head -1; echo "ps=${PIPESTATUS[0]}"; cat err | head -3
printf 'x := "1"\nr:\n  echo hi\n' > justfile
for f in --list --summary --dump --variables --groups --evaluate; do $J $f >/dev/full 2>err; echo "$f c=$?"; head -3 err|tail -2; done
$J r 2>/dev/full; echo "recipe stderr full c=$?"
t "--show space"
printf 'r:\n  echo hi\n' > justfile; rr --show ' '; rr --usage ' '
t "dep arg options"
cat > justfile <<'E'
[arg('z', long, value='on')]
c z='off' p='p':
  echo z={{z}} p={{p}}
e: (c 'bogus')
E
rr e; rr c bogus
cat > justfile <<'E'
[arg('y', long)]
a x='d' y:
  echo {{x}} {{y}}
b: (a '1')
E
rr b; rr --list
t "long chains parser"
python3 -c "print('x := ' + ' + '.join([\"'a'\"]*200000)); print('r:\n  echo')" > justfile; rr --list
python3 -c "print('x := ' + ' + '.join([\"'a'\"]*2000)); print('r:\n  echo')" > justfile; rr --list
python3 -c "print('x := ' + ''.join(['if \"a\"==\"b\" {\"c\"} else ']*100000)+'\"d\"'); print('r:\n  echo')" > justfile; rr --list
python3 -c "print('x := ' + '/ '*200000 + '\"a\"'); print('r:\n  echo')" > justfile; rr --list
python3 -c "print('x := ' + '('*300 + '\"a\"'+')'*300); print('r:\n  echo')" > justfile; HL=2 rr --list
t "resolver chains"
python3 -c "
N=60000
for i in range(N): print(f'a{i} := a{i+1}')
print(f'a{N} := \"x\"')" > justfile; rr --list
python3 -c "
N=30000
for i in range(N): print(f'r{i}: r{i+1}')
print(f'r{N}:')" > justfile; rr --list
python3 -c "
N=5000
for i in range(N): print(f'r{i}: r{i+1}')
print(f'r{N}:')" > justfile; rr --summary | cut -c1-40
t "const memory"
python3 -c "
N=35
for i in range(N): print(f'a{i} := a{i+1} + a{i+1}')
print(f'a{N} := \"x\"')
print('r:\n  echo')" > justfile; ( ulimit -v 2000000; timeout 60 $J --list >/dev/null 2>.e; echo c=$?; tail -2 .e | cut -c1-200)
t "unterminated fmt"
printf "x := f'{{'a'}} tail\n" > justfile; rr --list
printf "x := f'unterminated\n" > justfile; rr --list
t "module name file"
mkdir a; cd a; printf 'mod build\n' > justfile; echo 'r:' > build.just; echo '#!/bin/sh' > build; rr --list; cd ..
mkdir b; cd b; printf 'mod? bin\nr:\n' > justfile; touch bin; rr --list; printf 'import? "f/x.just"\nr:\n' > justfile; touch f; rr --list; printf 'mod? a "f/y"\nr:\n' > justfile; rr --list; cd ..
t "two modules one file"
printf 'mod a "x.just"\nmod b "x.just"\n' > justfile
cat > x.just <<'E'
v := "def"
r:
  echo mod={{module_path()}} v={{v}}
E
rr a::r; rr --summary; rr a::r b::r; rr a::v=9 a::r; rr b::v=9 a::r
t "module+import"
printf 'mod m "x.just"\nimport "x.just"\n' > justfile
printf 'r:\n  echo mp={{module_path()}}\n' > x.just; rr --summary; rr m r
t "shared import"
mkdir a b; printf 'mod a\nmod b\n' > justfile; printf 'import "../common.just"\nr:\n  echo {{module_path()}} v={{v}}\n' > a/mod.just; cp a/mod.just b/mod.just; printf 'v := "d"\n' > common.just; rr a::v=9 b::r; rr a::r
t "variable resolves parent/constant"
mkdir m; printf "mod m\ny := 'parent'\n" > justfile; printf "x := y\ny := 'child'\nr:\n  echo x={{x}} y={{y}}\n" > m/mod.just; rr m::r
printf "A := HEX\nHEX := 'mine'\nr:\n  echo A={{A}} HEX={{HEX}}\n" > justfile; rr r
printf "HEX := 'mine'\nZ := HEX\nr:\n  echo Z={{Z}} HEX={{HEX}}\n" > justfile; rr r
t "module set export"
mkdir m; printf "mod m\n_secret := 'x'\nplain := 'y'\n" > justfile; printf 'set export\nr:\n  echo $_secret $plain\n' > m/mod.just; rr m::r
t "module set lazy"
mkdir m; printf "mod m\ntop:\n  echo top\n" > justfile; printf 'set lazy\nx := `echo ran >&2; exit 4`\nr:\n  echo r\n' > m/mod.just; rr m::r; rr top
printf "set lazy\nmod m\ntop:\n  echo top\n" > justfile; printf 'x := `echo ran >&2; exit 4`\nr:\n  echo r\n' > m/mod.just; rr m::r; rr top
printf "mod m\ntop:\n  echo top\n" > justfile; rr top
t "alias list"
mkdir m; printf "mod m\nalias x := m::foo\nfoo:\n  echo local\n" > justfile; printf 'foo:\n  echo modfoo\nbar:\n' > m/mod.just; rr --list; rr x; rr --show x
rr --dump --dump-format json | python3 -c "import json,sys; l=sys.stdin.read().split('\n'); print(json.loads('\n'.join(l[1:-2]))['aliases'])"
printf "mod m\nalias x := m::bar\nd: m::bar (m::foo)\n" > justfile; rr --list; rr --alias-style separate --list;  rr --show d
t "circular mod"
printf 'mod x "../%s/justfile"\n' $n > justfile; rr --list 2>&1 | cut -c1-150
printf 'mod x "sub/../justfile"\n' > justfile; mkdir -p sub;  rr --list 2>&1 | cut -c1-150
printf 'mod x "justfile"\n' > justfile; rr --list
printf 'import "sub/../justfile"\n' > justfile; rr --list
t "arg count usage"
mkdir sub; printf 'mod sub\n' > justfile; printf 'r x:\n  echo {{x}}\n' > sub/mod.just; rr sub r; rr sub::r
t "multiline module doc list"
printf '[doc("l1\nl2")]\nmod m\n\n[doc("r1\nr2")]\nr:\n' > justfile; echo 'x:' > m.just; rr --list
t "alias to os-disabled"
printf '[windows]\nfoo:\n  echo\nalias f := foo\nother:\n  echo other\n' > justfile; rr other
printf '[windows]\nfoo:\n  echo\nall: foo\nother:\n  echo other\n' > justfile; rr other; rr --list
t "dotenv"
printf 'FOO=fromdotenv\n' > .env; printf 'set dotenv-path := "missing.env"\nset dotenv-required\nr:\n  echo $FOO\n' > justfile; rr r
printf 'set dotenv-path := "missing.env"\nr:\n  echo $FOO\n' > justfile; rr r
printf 'r:\n  echo ${FOO-unset}\n' > justfile; rr --dotenv-path nope.env r ; rr r
mkdir -p m; printf 'mod m\nr:\n  echo root=$X\n' > justfile; printf 'r:\n  echo m=$X\n' > m/mod.just; printf 'X=default\n' > .env; printf 'X=prod\n' > .env.prod; rr -E .env.prod m::r; rr -E .env.prod r
t "dotenv wd"
mkdir sub; printf 'V=root\n' > .env; printf 'V=sub\n' > sub/.env; printf 'set working-directory := "sub"\nset dotenv-path := ".env"\nr:\n  pwd; echo $V\n' > justfile; rr r
rm .env; printf 'set working-directory := "sub"\nset dotenv-load\nr:\n  pwd; echo ${V-unset}\n' > justfile; rr r
t "[env]"
cat > justfile <<'E'
export FOO := 'exported'
[env('FOO','attr')]
l:
  echo $FOO
[env('FOO','attr')]
s:
  #!/bin/sh
  echo $FOO
E
rr l; rr s
printf 'set dotenv-load\n[env("X","attr")]\nl:\n  echo $X\n' > justfile; echo X=dot > .env; rr l
cat > justfile <<'E'
[env('FOO','attr')]
l p=`echo ${FOO-unset}` q=shell('echo ${FOO-unset}'):
  echo p={{p}} q={{q}} body={{`echo ${FOO-unset}`}} sh={{shell('echo ${FOO-unset}')}} live=$FOO
E
rr l
t "dry run"
printf "x := shell('touch shell-ran; echo y')\ny := \`touch bt-ran; echo z\`\nr:\n  echo {{x}} {{y}}\n" > justfile; rr -n r; ls
rr --dry-run --command touch cmd-ran; ls
printf "[arg('x', pattern='a+')]\nr x=\`echo a\`:\n  echo {{x}}\n" > justfile; rr -n r; rr r
printf "x := assert(\`echo a\` == 'a','no')\nr:\n  echo {{x}}\n" > justfile; rr -n r; rr r
printf "[confirm]\nr:\n  echo hi\n" > justfile; rr -n r
t quiet
printf "#\ns:\n  #!/bin/sh\n  echo out; echo err >&2\n[script('sh')]\nt:\n  echo tout\nl:\n  echo lout; echo lerr >&2\n@q:\n  #!/bin/sh\n  echo qq\n" > justfile; rr --quiet s; rr --quiet t; rr --quiet l; rr --quiet --timestamp l; rr --quiet --time l; printf "# docdoc\nl:\n  echo x\n" > justfile;  rr --quiet --explain l
printf 'set quiet\n@a:\n  #!/bin/sh\n  echo body\nb:\n  #!/bin/sh\n  echo body2\n@c:\n  echo cc\n' > justfile; rr a; rr b; rr c
printf '@a:\n  #!/bin/sh\n  echo body\n' > justfile; rr a
