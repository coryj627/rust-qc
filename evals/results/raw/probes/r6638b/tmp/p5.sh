#!/bin/bash
J=/target/debug/just
export PATH=/target/debug:$PATH
t() { name=$1; shift
  d=$(mktemp -d /work/tmp/p/XXXXXX); cat > $d/justfile
  echo "=== $name :: just $*"
  (cd $d; timeout 20 $J "$@" 2>&1; echo "[exit $?]"; ls -A | grep -v justfile | sed 's/^/created: /')
}
d=$(mktemp -d /work/tmp/p/XXXXXX); printf 'a:\n  @echo ok\n' > $d/justfile; (cd $d; echo "== env submods"; JUST_LIST_SUBMODULES=true $J a 2>&1 | head -4;  JUST_CHECK=true $J a | head -3; JUST_DRY_RUN=true JUST_QUIET=true $J a 2>&1 | head -3)
# deep threshold
for n in 2000 5000 10000 20000 50000; do python3 -c "print('x := ' + ' + '.join([\"'a'\"]*$n) + '\nr:\n  @echo {{x}}\n')" > /work/tmp/p/deep.just; timeout 20 $J -f /work/tmp/p/deep.just --evaluate x >/dev/null 2>/work/tmp/p/deep.err; echo "n=$n exit=$? $(head -c 100 /work/tmp/p/deep.err | tr '\n' ' ')"; done
# Parenthesis depth limit check
t list_groups_mod --list <<'E'
[group('b')]
b:
[group('a')]
a:
c:
E
t unsorted_groups --list --unsorted <<'E'
[group('z')]
z:
[group('a')]
a:
E
t eval_private --evaluate <<'E'
[private]
export p := 'pv'
_q := 'q'
longlonglonglonglonglong := 'a'
[private]
averyveryveryveryveryveryveryverylong := 'x'
s := 's'
E
t eval_private_shell --evaluate --evaluate-format shell <<'E'
a-b := "x"
E
t var_list --variables <<'E'
b := 'b'
[private]
a := 'a'
E
t missing_doc_mod --list <<'E'
# module doc
mod? absent
a:
E
d=$(mktemp -d /work/tmp/p/XXXXXX); printf '# module doc\nmod sub\n[private]\nmod hid\n' > $d/justfile; printf 'r:\n' > $d/sub.just; printf '_p:\ns:\n' > $d/hid.just; (cd $d; echo "== list private module";  $J --list 2>&1; $J --list --list-submodules 2>&1; $J --summary; $J --groups; $J --list hid; echo "[exit $?]"; $J --usage hid s; echo; JUST_COMPLETE=bash _CLAP_COMPLETE_INDEX=1 _CLAP_COMPLETE_COMP_TYPE=9 _CLAP_COMPLETE_SPACE=true $J -- just "" 2>&1 | head -10;)
t choose_private --chooser cat --choose <<'E'
_a:
b:
[private]
c:
E
t default_attr <<'E'
a:
  @echo a
[default]
b:
  @echo b
E
t recipe_mod_same_name <<'E'
mod? foo
foo:
  @echo recipe
E
d=$(mktemp -d /work/tmp/p/XXXXXX); printf 'mod foo\nfoo:\n  @echo recipe\n' > $d/justfile; printf 'r:\n  @echo inner\n' > $d/foo.just; (cd $d; echo "== mod+recipe same name"; $J foo 2>&1; echo "[exit $?]")
d=$(mktemp -d /work/tmp/p/XXXXXX); printf 'mod foo\nx := "root"\n' > $d/justfile; printf 'x := "inner"\ny := x\nr:\n  @echo {{y}}\n' > $d/foo.just; (cd $d; echo "== override root only"; $J x=over foo r 2>&1; $J --set foo::x over foo r 2>&1; $J --set foo::nope over foo r 2>&1; echo "[exit $?]"; $J foo::x=over foo::r 2>&1)
t override_settings x=fish a <<'E'
x := 'sh'
set shell := [x, '-c']
a:
  echo hi
E
t set_const_eval a <<'E'
x := `echo sh`
set shell := [x, '-c']
a:
  echo hi
E
t set_const_var_order a <<'E'
a1 := b1 + 'x'
b1 := `echo sh`
set tempdir := 'foo' + a1
a:
  echo hi
E
t set_uses_fn a <<'E'
set tempdir := env('HOME')
a:
  echo hi
E
t set_is_default a <<'E'
set dotenv-load := true
set allow-duplicate-variables := false
a:
  echo hi
E
t assert_set a <<'E'
set tempdir := assert('a' == 'b', 'fail')
a:
  echo hi
E
t regex_err a <<'E'
x := if 'a' =~ '(' { 'y' } else { 'n' }
a:
  echo {{x}}
E
t cond_order a <<'E'
y := 'a' + 'b' / 'c' + 'd'
z := / 'x'
w := 'p' + / 'q'
v := 'a/' / 'b'
a:
  @echo {{y}} {{z}} {{w}} {{v}}
E
t params_dep_args a <<'E'
a: (b 'x' 'y' 'z')
b p +q:
  @echo p={{p}} q={{q}}
E
t dep_args_dedup a <<'E'
a: (b 'x y') (b 'x' )
b +q:
  @echo q={{q}}
E
t dep_group_dedup a <<'E'
a: (b 'x y') (c 'x' 'y')
b *q:
  @echo b q={{q}}
c *q: (b q)
  @echo c q={{q}}
E
t subseq_ran a <<'E'
a: c && b c
b:
  @echo b
c:
  @echo c
E
t parallel_fail a <<'E'
[parallel]
a: b c
b: d
c: d
d:
  @echo d-ran
  @exit 1
E
t dep_error_exit a <<'E'
a: b
b:
  @exit 3
E
t script_fail_message a <<'E'
a:
  #!/bin/sh
  exit 3
E
t no_exit_msg_script a <<'E'
[no-exit-message]
a:
  #!/bin/sh
  exit 3
E
t line_num a <<'E'
a:
  @echo a \
    b

  # c

  @exit 4
E
t line_num_cont_comment a <<'E'
set ignore-comments
a:
  # c \
  @exit 4
E
t ignore_comment_interp a <<'E'
set ignore-comments
a:
  # {{ error('no') }}
  @echo ok
E
t shebang_err_line a <<'E'


a:
  #!/usr/bin/env python3
  import sys
  # x

  raise Exception('boom')
E
t script_attr_err_line a <<'E'

[script('python3')]
a:
  import sys

  raise Exception('boom')
E
t shebang_env_args a <<'E'
a:
  #!/usr/bin/env -S sh -eu
  echo $0 | sed 's/.*\///'
E
t ext a <<'E'
[extension('.foo')]
a:
  #!/bin/sh
  basename "$0"
E
t script_empty a <<'E'
[script]
a:
E
t script_noattr_shebang_empty a <<'E'
a:
  #!
  echo
E
