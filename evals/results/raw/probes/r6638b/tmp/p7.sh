#!/bin/bash
J=/target/debug/just
export PATH=/target/debug:$PATH
t() { name=$1; shift
  d=$(mktemp -d /work/tmp/p/XXXXXX); cat > $d/justfile
  echo "=== $name :: just $*"
  (cd $d; timeout 20 $J "$@" 2>&1; echo "[exit $?]"; ls -A | grep -v justfile | sed 's/^/created: /')
}
t choose_group --choose --group g --chooser cat <<'E'
[group('g')]
a:
  @echo a
b:
  @echo b
E
d=$(mktemp -d /work/tmp/p/XXXXXX); printf "[group('g')]\na:\n  @echo a\nb:\n  @echo b\n[group('g')]\nmod m\n" > $d/justfile; printf 'c:\n  @echo c\n' > $d/m.just; (cd $d; echo "== choose JUST_GROUP"; JUST_GROUP=g $J --choose --chooser cat 2>&1; echo "[exit $?]")
t show_space --show ' ' <<'E'
a:
E
t usage_space --usage ' ' <<'E'
a:
E
d=$(mktemp -d /work/tmp/p/XXXXXX); printf 'a:\n' > $d/.justfile; (cd $d; echo "== init with .justfile"; $J --init 2>&1; echo "[exit $?]"; ls -A;  $J --list 2>&1 | head -3)
t fn_dyn_scope --unstable r <<'E'
x := 'global'
y := x
f(x) := y
r:
  @echo {{f('param')}} {{y}}
E
t fn_dup_param --unstable r <<'E'
f(a, a) := a
r:
  @echo {{f('1', '2')}}
E
t fn_backtick_twice --unstable r <<'E'
x := `echo ran >&2; echo v`
f() := x
r:
  @echo {{f()}} {{f()}} {{x}}
E
t fn_const_param --unstable r <<'E'
f(HEX) := HEX
r:
  @echo {{f('1')}}
E
t fn_in_set --unstable r <<'E'
f() := 'a'
set tempdir := f()
r:
  @echo hi
E
t fn_undefined_in_body --unstable <<'E'
f() := nope
r:
E
t recipe_param_undefined_order r <<'E'
r a=b b='1':
  echo {{a}}
E
t mod_list_group --list --group g <<'E'
[group('g')]
mod? nope
a:
E
t comments_misc --fmt <<'E'
set quiet # c1
import? 'nf' # c2
mod? m # c3
unexport X # c4
alias a := b # c5
x := 'y' # c6
b: # c7
c: b # c8
f() := 'a' # c9
E
t empty_dep_paren r <<'E'
r: (a)
a:
E
t dep_trailing_comment r <<'E'
r: a # comment
  @echo body
a:
E
t recipe_doc_empty --list <<'E'
#
a:
E
t doc_not_adjacent --list <<'E'
# not doc

a:
E
t doc_after_item --list <<'E'
x := 'a' # trailing
a:
b: # t
c:
E
t doc_attr_cooked --list <<'E'
[doc("a\tb")]
a:
E
t alias_private_target --list <<'E'
alias pub := _hidden
_hidden:
E
t alias_summary --summary <<'E'
alias x := a
a:
E
