#!/bin/bash
J=/target/debug/just
export PATH=/target/debug:$PATH
t() { # name, justfile content from stdin, then args
  name=$1; shift
  d=$(mktemp -d /work/tmp/p/XXXXXX); cat > $d/justfile
  echo "=== $name :: just $*"
  (cd $d; timeout 20 $J "$@" 2>&1; echo "[exit $?]")
}
t datetime_bad <<'E'
x := datetime('%Q')
a:
  echo {{x}}
E
t datetime_utc_bad <<'E'
a:
  echo {{datetime_utc('%Q')}}
E
d=$(mktemp -d /work/tmp/p/XXXXXX); printf 'a:\n  echo hi\n' > $d/justfile; (cd $d; echo "== timestamp fmt"; $J --timestamp --timestamp-format '%Q' a 2>&1; echo "[exit $?]")
t recursive_fn --unstable <<'E'
f(x) := f(x)
a:
  echo {{f('1')}}
E
t cycle_fn --unstable <<'E'
x := f()
f() := x
a:
  echo {{x}}
E
t fn_override --unstable x=over a <<'E'
x := 'orig'
f() := x
a:
  echo {{x}} {{f()}}
E
t fn_export --unstable a <<'E'
export A := 'val'
f() := `echo $A`
a:
  echo {{f()}}
E
t fn_is_dep --unstable b <<'E'
f() := is_dependency()
a:
  echo {{is_dependency()}} {{f()}}
b: a
E
t dep_arg_is_dep b <<'E'
b: (c is_dependency())
c x:
  echo {{x}}
E
t confirm_is_dep --yes b <<'E'
[confirm(is_dependency())]
b:
  echo hi
E
echo n | t confirm_is_dep2 b 2>&1 <<'E'
[confirm(is_dependency())]
b:
  echo hi
E
t const_shadow a <<'E'
a := RED
RED := 'mine'
a:
  echo '[{{a}}]'
E
d=$(mktemp -d /work/tmp/p/XXXXXX); cat > $d/justfile <<'E'
z := 'parent'
mod sub
E
cat > $d/sub.just <<'E'
a := z
z := 'child'
r:
  echo a={{a}} z={{z}}
E
(cd $d; echo "== mod shadow"; $J sub r 2>&1; echo "[exit $?]")
t ignore_comments_script a <<'E'
set ignore-comments
a:
  #!/bin/sh
  # {{undefined}}
  echo hi
E
t ignore_comments_cont a <<'E'
set ignore-comments
a:
  echo a \
  # {{undefined}}
E
t env_attr_export a <<'E'
export FOO := 'exported'
[env('FOO', 'attr')]
a:
  echo $FOO
E
t env_attr_backtick a <<'E'
[env('FOO', 'attr')]
a x=`echo $FOO`:
  echo {{x}} {{`echo $FOO`}} {{shell('echo $FOO')}}
E
t param_export_backtick a <<'E'
a $x='1' y=`echo $x`:
  echo {{y}}
E
t eval_shell --evaluate --evaluate-format shell <<'E'
x := 'hello!'
E
d=$(mktemp -d /work/tmp/p/XXXXXX); printf "x := 'hello!'\ny := 'a\"b\$c'\n" > $d/justfile; (cd $d; echo "== eval roundtrip"; $J --evaluate --evaluate-format shell; eval "$($J --evaluate --evaluate-format shell)"; echo "x=[$x] y=[$y]")
t quiet_script --quiet a <<'E'
a:
  #!/bin/sh
  echo should-be-suppressed
  echo err >&2
E
t quiet_line --quiet a <<'E'
a:
  echo should-be-suppressed
E
t quiet_timestamp --quiet --timestamp a <<'E'
a:
  @echo hi
E
t at_timestamp --timestamp a <<'E'
a:
  @echo hi
E
t quiet_time --quiet --time a <<'E'
a:
  @echo hi
E
