J=/target/debug/just
T=/tmp/p; mkdir -p $T; cd $T
run() { echo "--- $1"; shift; "$@" 2>&1 | head -${LINES_MAX:-14}; echo "rc=${PIPESTATUS[0]}"; }
mk() { rm -rf $T/w; mkdir -p $T/w; cd $T/w; }
# env attr visibility
mk; printf '[env("FOO","attr")]\nr p=`echo def=$FOO`:\n  echo {{p}} {{ `echo body=$FOO` }} {{ shell("echo sh=$FOO") }} run=$FOO\n' > justfile; run env-backticks $J r
mk; printf '[env("FOO","attr")]\nunexport FOO\nr:\n  echo [${FOO-unset}]\n' > justfile; run env-unexport $J r
# dotenv with working-directory setting
mk; mkdir sub; printf 'K=sub\n' > sub/.env; printf 'K=root\n' > .env; printf 'set working-directory := "sub"\nset dotenv-path := ".env"\nr:\n  pwd; echo $K\n' > justfile; run dotenv-wd $J r
mk; mkdir sub; printf 'K=sub\n' > sub/.env; printf 'set working-directory := "sub"\nset dotenv-load\nr:\n  pwd; echo ${K-unset}\n' > justfile; run dotenv-wd2 $J r
# module cycles via unclean path
mk; printf 'mod x "../w/justfile"\nr:\n' > justfile; run mod-cycle-unclean $J --list
mk; mkdir sub; printf 'mod x "sub/../justfile"\nr:\n' > justfile; run mod-cycle-unclean2 $J --list
mk; ln -s . link; printf 'import "link/justfile"\nr:\n' > justfile; run import-symlink-cycle $J --list
# duplicates function parameter
mk; printf 'f(a, a) := a\nr:\n  echo {{ f("1","2") }}\n' > justfile; run fn-dup-param $J --unstable r
# functions with assignment scope
mk; printf 'export E := "exp"\nf() := `echo ${E-unset}`\nr:\n  echo {{ f() }} {{ `echo ${E-unset}` }}\n' > justfile; run fn-export-in-backtick $J --unstable r
mk; printf 'f() := `echo hi`\nr:\n  echo {{ f() }}\n' > justfile; run fn-dry $J --unstable --dry-run r
mk; printf 'g(x) := x\nf(y) := g(y) + os()\nv := f("q")\nr a=f("d"):\n  echo {{a}} {{v}}\nmod? m\n' > justfile; run fn-default $J --unstable r
mk; mkdir m; printf 'f() := "root"\nmod m\n' > justfile; printf 'r:\n  echo {{ f() }}\n' > m/mod.just; run fn-module-scope $J --unstable m::r
mk; printf 'set shell := [f()]\nf() := "sh"\nr:\n' > justfile; run fn-in-set $J --unstable r
mk; printf 'os() := "custom"\nr:\n  echo {{os()}}\n' > justfile; run fn-shadow $J --unstable r
# signals with --edit/--choose
mk; printf 'r:\n' > justfile; printf '#!/bin/sh\nsleep 3; echo editor-done > /tmp/p/edited\n' > /tmp/p/ed; chmod +x /tmp/p/ed; rm -f /tmp/p/edited; (VISUAL=/tmp/p/ed $J --edit & P=$!; sleep 0.5; kill -INT $P; sleep 0.2; if kill -0 $P 2>/dev/null; then echo just-still-alive; else echo just-exited; fi; wait $P; echo "rc=$?"; pgrep -f /tmp/p/ed | wc -l; )
(printf 'r:\n  echo r\n' > justfile; $J --choose --chooser "sleep 3; echo r" & P=$!; sleep 0.5; kill -TERM $P; sleep 0.2; if kill -0 $P 2>/dev/null; then echo just-still-alive; else echo just-exited; fi; wait $P; echo "rc=$?"; pgrep -f 'sleep 3' | wc -l)
# exit codes after signal
mk; printf 'r:\n  sleep 5\n  echo notreached\n' > justfile; ($J r & P=$!; sleep 0.5; kill -TERM $P; wait $P; echo "rc=$?")
# interrupted backtick
mk; printf 'x := `sleep 5; echo x`\nr:\n  echo {{x}}\n' > justfile; ($J r & P=$!; sleep 0.5; kill -TERM $P; wait $P; echo "rc=$?")
# subsequents
mk; printf 'c:\n  echo c\nb: c\n  echo b\na: c && b\n  echo a\n' > justfile; run subseq $J a
mk; printf 'c:\n  echo c\na: && c\n  echo a\nd: a c\n  echo d\n' > justfile; run subseq2 $J d
# aliases in JSON
mk; mkdir m; printf 'mod m\nalias x := m::foo\n' > justfile; printf 'foo:\n' > m/mod.just; $J --dump --dump-format json | python3 -c 'import json,sys; d=json.load(sys.stdin); print(d["aliases"])'; run show-alias $J --show x; run usage-alias $J --usage x
# overrides
mk; printf 'x := "a"\ny := x + "b"\nset shell := ["sh", "-c" + x]\nr:\n  echo {{y}}\n' > justfile; run set-shell-override $J x=u r
mk; printf 'export x := "a"\nr:\n  echo $x {{x}}\n' > justfile; run override-export $J x=b r
mk; mkdir m; printf 'mod m\n' > justfile; printf 'x := "a"\nr:\n  echo {{x}}\n' > m/mod.just; run override-mod $J --set m::x q m::r; run override-mod2 $J m::x=q m::r; run override-mod3 $J m::nope=q m::r; run override-mod4 $J nomod::nope=q m::r
# evaluate multi
mk; printf 'x := "a"\n_p := "secret"\n[private]\nq := "s"\nexport y := "é"\n' > justfile; run eval-all $J --evaluate; run eval-private $J --evaluate _p; run eval-var $J --variables
# quiet / verbose
mk; printf 'x := `echo bad >&2; exit 3`\nr:\n  echo {{x}}\n' > justfile; run backtick-fail $J r; run backtick-fail-q $J --quiet r
mk; printf 'x := `printf "\\377"`\nr:\n  echo {{x}}\n' > justfile; run backtick-nonutf8 $J r
# shebang args
mk; printf 'r:\n  #!/usr/bin/env -S sh -eu\n  echo {{"x"}} $0\n' > justfile; run shebang-env $J r
mk; printf 'r a="1":\n  #!/bin/sh\n  echo "$1|$#"\n[positional-arguments]\ns a="1":\n  #!/bin/sh\n  echo "$1|$#"\n' > justfile; run script-pos $J s Z
mk; printf '[no-cd]\nr:\n  pwd\n  #!foo\n' > justfile; mkdir d; cd d; run no-cd $J r; cd ..
mk; printf '[extension("/x.sh")]\nr:\n  #!/bin/sh\n  echo $0\n' > justfile; run ext-slash $J r
mk; printf 'r:\n  #!/bin/sh\n  echo $0\n' > justfile; run script-tempdir $J --tempdir nonexist r
# tempdir noexec fallback / XDG_RUNTIME unwritable
mk; printf 'r:\n  #!/bin/sh\n  echo $0\n' > justfile; XDG_RUNTIME_DIR=/proc/nonexistent run xdg-bad $J r
# recipe name + file
mk; printf 'a b:\n  echo {{b}}\n' > justfile; run arg-dash $J a -- ; run arg-dash2 $J a -x
mk; printf '[arg("x", long)]\na x="d" y="e":\n  echo {{x}} {{y}}\n' > justfile; run opt1 $J a --x=1 2; run opt2 $J a -- -z; run opt3 $J a -- a b; run opt4 $J a 1 --x 3; run opt5 $J a --x; run opt6 $J a --x= ;run opt7 $J a - 
mk; printf '[arg("x", long, value="v")]\na x:\n  echo {{x}}\n' > justfile; run flag-required $J a; run flag-on $J a --x; run usage-flag $J --usage a
mk; printf '[arg("x", short="é")]\na x:\n  echo {{x}}\n' > justfile; run short-unicode $J a -é 1;  run short-multi $J a -éx 1
