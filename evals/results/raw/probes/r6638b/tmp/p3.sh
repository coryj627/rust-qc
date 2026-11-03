#!/bin/bash
J=/target/debug/just
export PATH=/target/debug:$PATH
t() { name=$1; shift
  d=$(mktemp -d /work/tmp/p/XXXXXX); cat > $d/justfile
  echo "=== $name :: just $*"
  (cd $d; timeout 20 $J "$@" 2>&1; echo "[exit $?]")
}
t const_shadow2 r <<'E'
A := RED
RED := 'mine'
r:
  echo '[{{A}}]'
E
# dotenv path missing fallback
d=$(mktemp -d /work/tmp/p/XXXXXX); echo 'FOO=from-dot-env' > $d/.env; printf 'set dotenv-path := "missing.env"\nset dotenv-required\na:\n  echo FOO=$FOO\n' > $d/justfile; (cd $d; echo "== dotenv-path missing"; $J a 2>&1; echo "[exit $?]"; printf 'a:\n  echo FOO=$FOO\n' > justfile; $J --dotenv-path nope.env a 2>&1; echo "[exit $?]")
# choose: multi-select with optional arg
d=$(mktemp -d /work/tmp/p/XXXXXX); printf 'a x="dflt":\n  @echo a x={{x}}\nb:\n  @echo b\n[private]\nmod? p\n' > $d/justfile; printf 'r:\n  @echo private-module-recipe\n' > $d/p.just; (cd $d; echo "== choose"; $J --chooser 'cat' --choose 2>&1; echo "[exit $?]")
# list alias into submodule
d=$(mktemp -d /work/tmp/p/XXXXXX); printf 'mod sub\nalias f := sub::build\nbuild:\n  echo root\ntest: sub::build (sub::arg "1")\n' > $d/justfile; printf 'build:\n  echo sub\narg x:\n  echo {{x}}\n' > $d/sub.just; (cd $d; echo "== list alias"; $J --list 2>&1;  echo "-- show test"; $J --show test 2>&1; echo "-- show f"; $J --show f 2>&1; echo '-- show "sub build"'; $J --show 'sub build' 2>&1; echo "[exit $?]";  echo "-- show sub build";$J --show sub build 2>&1; echo "[exit $?]"; echo '-- json alias'; $J --dump --dump-format json | python3 -c 'import json,sys; d=json.load(sys.stdin); print(d["aliases"], d["recipes"]["test"]["dependencies"])'; echo "-- no local"; printf 'mod sub\nalias f := sub::build\n' > justfile; $J --list 2>&1;  $J --alias-style separate --list 2>&1)
# which on directory
d=$(mktemp -d /work/tmp/p/XXXXXX); mkdir -p $d/bin/foo; printf 'a:\n  @echo "[{{which("foo")}}]"\n' > $d/justfile; (cd $d; echo "== which dir"; PATH=$d/bin:$PATH $J --unstable a 2>&1; echo "[exit $?]")
# imports dup
d=$(mktemp -d /work/tmp/p/XXXXXX); printf 'set allow-duplicate-recipes\nset allow-duplicate-variables\nimport "a.just"\nimport "b.just"\n' > $d/justfile; printf 'v := "a"\nr:\n  @echo a {{v}}\n' > $d/a.just; printf 'v := "b"\nr:\n  @echo b {{v}}\n' > $d/b.just; (cd $d; echo "== imports dup"; $J r 2>&1; echo "[exit $?]")
# ceiling relative
d=$(mktemp -d /work/tmp/p/XXXXXX); mkdir -p $d/x/y; printf 'a:\n  @echo found\n' > $d/justfile; (cd $d/x/y; echo "== ceiling"; $J --ceiling .. a 2>&1; echo "[rel exit $?]"; $J --ceiling $d/x a 2>&1; echo "[abs exit $?]")
t groups_args --groups foo bar <<'E'
a:
E
t completions_args --completions bash foo <<'E'
E
# backticks with no-cd / working-directory
d=$(mktemp -d /work/tmp/p/XXXXXX); mkdir -p $d/w $d/inv; printf '[working-directory("w")]\na x=`pwd`:\n  @echo x={{x}} body={{`pwd`}} real=$(pwd) shell={{shell("pwd")}} abs={{absolute_path(".")}} read={{path_exists("marker")}}\n[no-cd]\nb:\n  @echo body={{`pwd`}} real=$(pwd)\n' > $d/justfile; touch $d/w/marker; (cd $d/inv; echo "== wd"; $J a 2>&1; $J b 2>&1)
t deep_plus r < <(python3 -c "print('x := ' + ' + '.join([\"'a'\"]*200000) + '\nr:\n  echo\n')")
t deep_slash r < <(python3 -c "print('x := ' + '/ '*200000 + \"'a'\" + '\nr:\n  echo\n')")
t deep_elif r < <(python3 -c "print('x := ' + \"if 'a' == 'b' { 'c' } else \"*100000 + \"{ 'z' }\" + '\nr:\n  echo\n')")
t choose_empty --chooser true --choose <<'E'
a:
  echo a
E
t args_dash a -x <<'E'
a x:
  echo {{x}}
E
t opt_positional a -- --foo <<'E'
[arg('o', long)]
a x o='d':
  echo {{x}} {{o}}
E
t opt_value_eq a --o=1=2 v <<'E'
[arg('o', long)]
a x o='d':
  echo {{x}} {{o}}
E
t opt_dup_flag a --f --f <<'E'
[arg('f', long, value='y')]
a f='n':
  echo {{f}}
E
t usage_show --usage a <<'E'
[arg('o', long, short='s', help='H')]
[arg('f', long, value='y')]
[arg('p', pattern='a|b')]
a p o='d' f='n' *rest:
  echo
E
t pattern_alt a b c <<'E'
[arg('x', pattern='a|b')]
a x y:
  echo {{x}} {{y}}
E
t json_pattern --dump --dump-format json <<'E'
[arg('x', pattern='a|b')]
a x:
  echo
E
t positional_opt a v --o w <<'E'
set positional-arguments
[arg('o', long)]
a x o='d':
  echo "$0 $1 $2"
E
t eval_var --evaluate y <<'E'
x := `echo ran-x >&2; echo x`
y := 'a'
E
t eval_quote --evaluate <<'E'
x := 'a"b'
E
t variadic_default a <<'E'
a +x='a b':
  echo "{{x}}"
E
t star_positional a <<'E'
set positional-arguments
a *x:
  @echo "$#"
E
t star_default_positional a <<'E'
set positional-arguments
a *x='d':
  @echo "$#"
E
