#!/bin/bash
. /work/tmp/lib.sh
t "x string env"
printf "x := x'\$UNSET_VAR_ZZ'\nr:\n  echo\n" > justfile; rr --unstable --fmt --check; rr --list; rr --dump
t "evaluate shell"
printf "x := 'a!b'\n" > justfile; rr --evaluate --evaluate-format shell; $J --evaluate --evaluate-format shell > e.sh; sh -c '. ./e.sh; echo "sh:$x"'; bash -c '. ./e.sh; echo "bash:$x"'
printf "x := 'a\"b'\n" > justfile; rr --evaluate; rr --evaluate x; rr --evaluate --evaluate-format just;
t parallel
cat > justfile <<'E'
[parallel]
a: b c
b: x
c: x
x:
  @echo running-x
  @false
E
rr a
t "lazy diamond"
python3 -c "
N=26
print('set lazy')
for i in range(N): print(f'a{i}: b{i} c{i}\nb{i}: a{i+1}\nc{i}: a{i+1}')
print(f'a{N}:\n  @echo end')" > justfile
s=$(date +%s.%N); timeout 30 $J a0 >/dev/null 2>&1; echo "lazy c=$? $(echo "$(date +%s.%N)-$s"|bc 2>/dev/null)"
python3 -c "
N=20
print('set lazy')
for i in range(N): print(f'a{i}: b{i} c{i}\nb{i}: a{i+1}\nc{i}: a{i+1}')
print(f'a{N}:\n  @echo end')" > justfile
time (timeout 30 $J a0 >/dev/null 2>&1; echo "lazy20 c=$?")
sed -i '/set lazy/d' justfile; time (timeout 30 $J a0 >/dev/null 2>&1; echo "nolazy20 c=$?")
t "unused const asserts"
printf 'set lazy\nx := assert("a" == "b", "unused")\nr:\n  echo r\n' > justfile; rr r; rr --list; rr --dump
printf 'x := error("nope")\nr:\n  echo r\n' > justfile; rr --list; rr r; 
printf 'set lazy\nx := error("nope")\nr:\n  echo r\n' > justfile; rr --list; rr r; 
printf 'set lazy\nx := if "a" =~ "(" {"b"} else {"c"}\nr:\n  echo r\n' > justfile; rr --list
t chooser
printf 'a x="dflt":\n  echo chosen-a {{x}}\nb:\n  echo chosen-b\n[private]\nmod p\n[group("g")]\nmod q\n' > justfile; echo 'sec:' > p.just; echo 'qr:' > q.just
rr --choose --chooser true
rr --choose --chooser 'cat'
rr --choose --chooser 'cat >&2; echo a; echo b'
rr --group g --choose --chooser 'cat >&2'
t "default chooser quote"
mkdir "it's"; cd "it's"; printf 'a:\n' > justfile; rr --choose; cd ..
t "ceiling"
mkdir -p sub/deep; printf 'r:\n  echo parent\n' > justfile; cd sub; rr --ceiling . r;  rr --ceiling $PWD r; cd deep; rr --ceiling .. r; JUST_CEILING=. rr r; cd ../..
t "ceiling fallback"
mkdir -p nest/x/y; printf 'root:\n  echo root\n' > nest/justfile; printf 'set fallback\nz:\n' > nest/x/justfile; cd nest/x/y; rr --ceiling $D/nest/x root; rr --help | grep -i -A2 ceiling; cd ../../..
t "is_dependency dep arg"
printf 'a: (b is_dependency())\nb x:\n  echo x={{x}} self={{is_dependency()}}\n[confirm(is_dependency())]\nc:\n  echo c\n' > justfile; rr a; rr c
t "indented fstrings"
cat > justfile <<'E'
v := "  deep\n shallow"
w := "A\nB"
p := f'''
    x
    {{ w }}
    y
'''
q := f'''

  a
'''
q2 := '''

  a
'''
r:
  @echo "[{{p}}]"
  @echo "[{{q}}]"
  @echo "[{{q2}}]"
E
rr r
t "line numbers"
printf 'r:\n  echo {{\n    "a"\n  }}\n  false\n' > justfile; rr r
printf 'a:\nb:\nfoo: a \\\n  b\n  echo 1\n  exit 3\n' > justfile; rr foo
printf '\n\n\nr:\n  true\n  false\n' > justfile; rr r
printf 'r:\n  true \\\n  x\n  false\n' > justfile; rr r
t "--command"
mkdir sub; printf 'set working-directory := "sub"\nr:\n  pwd\n' > justfile; rr r; rr --command pwd; rr --command sh -c 'exit 7'; rr --command sh -c 'kill -9 $$'; 
t "trailing backslashes"
printf 'r:\n  echo a\\\\\n  echo b\n' > justfile; rr r
t "--init"
touch .justfile; rr --init; ls -a; rr --list; rm justfile .justfile; touch Justfile; rr --init; ls -a; rr --list;
t "directory candidate"
mkdir -p sub/justfile; printf 'r:\n  echo p\n' > justfile; cd sub; rr r; cd ..; mkdir -p build/justfile; echo 'mod build' > justfile; rr --list; rm -rf build/justfile; 
t "option default"
printf '[arg("o", long)]\nr o="d" p:\n  echo {{o}} {{p}}\n' > justfile; rr r x
t "duplicate fn params"
printf 'set unstable\nf(a, a) := a\nr:\n  echo {{f("1","2")}}\n' > justfile; rr r
printf 'r a a:\n' > justfile; rr --list
t "guard signal"
cat > justfile <<'E'
set guards
a:
  ?sh -c 'kill -TERM $PPID; sleep 0.2; exit 1'
  echo after-guard
b: a
  echo after-r
E
rr b; grep -n -i guard /work/crate/README.md | head -5
t "json unexports"
printf 'unexport A\nunexport B\nunexport C\nunexport D\nunexport E\nunexport F\n' > justfile; for i in 1 2 3 4 5 6; do $J --dump --dump-format json | python3 -c "import json,sys; print(json.load(sys.stdin)['unexports'])"; done
t "empty shebang"
printf 'r:\n  #!{{ "" }}\n  echo\n' > justfile; rr r; printf 'r:\n  #!\n  echo\n' > justfile; rr r
t "mixed ws"
printf 'a:\n  b\n \tc\n' > justfile; rr --list; printf 'a:\n \tb\n \tc\n' > justfile; rr --list;
t "editor"
printf 'a:\n' > justfile; VISUAL="sh -c true" rr --edit; VISUAL= EDITOR=true rr --edit; EDITOR='true --wait' rr --edit; env -u VISUAL EDITOR=true $J --edit </dev/null; echo c=$?; 
t "signals"
cat > justfile <<'E'
a:
  kill -HUP $$; echo survived
E
nohup $J a </dev/null 2>&1 | tail -3; echo; nohup sh -c 'kill -HUP $$; echo sh-survived' </dev/null 2>&1 | tail -2
cat > j2 <<'E'
a:
  kill -INT $$; echo survived
E
bash -c 'trap "" INT; '$J' --justfile j2 a; echo c=$?' </dev/null 2>&1 | tail -3
t "script ext"
mkdir bin; printf '#!/bin/sh\necho "name=$1"\n' > bin/pwsh; chmod +x bin/pwsh; printf "[script('$D/bin/pwsh')]\na:\n  x\n" > justfile; rr a; printf "[script('pwsh')]\nb:\n  x\n" > justfile; PATH=$D/bin:$PATH rr b
t "missing wd"
printf '[working-directory("nope")]\nr:\n  echo\ns:\n  #!/bin/sh\n  echo\n' > justfile; rr r; printf 'set working-directory := "nope"\nr:\n  echo\n' > justfile; rr r; 
t "columns"
printf 'x := "éééééééé" + y\n' > justfile; rr --list
t "takes_arguments"
printf 'a:\n' > justfile; rr --groups extra; rr --choose extra --chooser true; rr --completions bash extra | tail -1; rr --summary extra;rr --variables extra; rr --dump extra
t "unexport"
printf "foo := 'a'\nunexport foo\nr:\n  echo\n" > justfile; rr r
printf "export foo := 'a'\nunexport foo\nr:\n  echo\n" > justfile; rr r
t "allow-missing fallback"
printf 'set fallback\nr:\n' > justfile; rr --allow-missing nope; printf 'r:\n' > justfile; rr --allow-missing nope
t "backtick wd"
mkdir w; cat > justfile <<'E'
[working-directory("w")]
r:
  echo {{`pwd`}} {{shell('pwd')}} $(pwd) {{absolute_path('.')}} {{path_exists('f')}}
[no-cd]
s:
  echo {{`pwd`}} $(pwd)
E
rr r; touch w/f; rr r; mkdir -p inv; cd inv; rr --justfile ../justfile --working-directory . s; cd ..
t "tempdir"
mkdir -p sub/tmpd; printf 'mod sub\n' > justfile; printf 'set tempdir := "tmpd"\ns:\n  #!/bin/sh\n  echo $0\n' > sub/mod.just; rr sub::s
t "JUST_ bool"
printf 'a:\n  echo a\n' > justfile; JUST_QUIET=1 rr a; JUST_YES=1 rr a; JUST_LIST_SUBMODULES=1 rr --list; JUST_QUIET=true rr a; JUST_UNSTABLE=1 rr a
t "metadata"
printf '[metadata("z")]\n[metadata("a")]\nr:\n' > justfile; rr --dump | head -4;  $J --dump --dump-format json | python3 -c "import json,sys; print(json.load(sys.stdin)['recipes']['r']['attributes'])"
t "circular dep"
printf 'a: b\nb: c\nc: b\n' > justfile; rr --list
t "--show dep"
mkdir m; printf 'mod m\nd: m::bar (m::baz "x")\n' > justfile; printf 'bar:\nbaz x:\n' > m/mod.just; rr --show d; $J --dump --dump-format json | python3 -c "import json,sys; print(json.load(sys.stdin)['recipes']['d']['dependencies'])"
