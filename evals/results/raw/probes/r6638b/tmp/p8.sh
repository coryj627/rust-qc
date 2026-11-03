#!/bin/bash
J=/target/debug/just
export PATH=/target/debug:$PATH
t() { name=$1; shift
  d=$(mktemp -d /work/tmp/p/XXXXXX); cat > $d/justfile
  echo "=== $name :: just $*"
  (cd $d; timeout 20 $J "$@" 2>&1; echo "[exit $?]"; ls -A | grep -v justfile | sed 's/^/created: /')
}
d=$(mktemp -d /work/tmp/p/XXXXXX); printf 'mod sub\n' > $d/justfile; printf 'r x:\n  @echo {{x}}\n' > $d/sub.just; (cd $d; echo "== usage in module error"; $J sub r 2>&1; echo "[exit $?]"; $J sub r a b 2>&1; echo "[exit $?]"; $J sub::nope 2>&1; echo "[exit $?]"; $J sub nope 2>&1; echo "[exit $?]")
t dry_pattern -n r <<'E'
[arg('x', pattern='a+')]
r x=`echo a`:
  echo {{x}}
E
t dry_assert -n r <<'E'
x := assert(`echo a` == 'a', 'no')
r:
  echo ok
E
t notdry_assert r <<'E'
x := assert(`echo a` == 'a', 'no')
r:
  @echo ok
E
t dry_confirm -n r <<'E'
[confirm]
r:
  echo ok
E
t metadata_order --dump --dump-format json <<'E'
[metadata('z')]
[metadata('a')]
r:
E
t parallel_sub a <<'E'
[parallel]
a: && b c
b:
  @echo b
c:
  @echo c
E
t bang_comment_doc --list <<'E'
#!/usr/bin/env just --justfile
a:
E
t env_interp_attr r <<'E'
[env('X', x'$HOME')]
r:
  @echo $X
E
t attr_group_order --fmt <<'E'
[private]
[group('b')]
[group('a')]
[linux]
r:
E
t string_newline_cont r <<'E'
x := "a\
   b"
r:
  @echo '{{x}}'
E
t if_regex_empty r <<'E'
x := if '' =~ '' { 'y' } else { 'n' }
r:
  @echo {{x}}
E
t choose_fn r <<'E'
r:
  @echo {{choose('0', '')}}. {{choose('3','a')}}
E
t semver r <<'E'
r:
  @echo {{semver_matches('1.2.3', '>=1')}} {{clean('')}} {{clean('a/../..')}} {{join('a','/b')}}  {{file_stem('a.b.c')}} {{without_extension('a.b/c')}} {{extension('.bashrc') || 'none'}}
E
t style r <<'E'
r:
  @echo {{style('command')}}x{{NORMAL}}
E
t which_empty --unstable r <<'E'
r:
  @echo [{{which('')}}]
E
t which_dot --unstable r <<'E'
r:
  @echo [{{which('.')}}] [{{which('/')}}] [{{which('./justfile')}}]
E
t require_msg r <<'E'
r:
  @echo [{{require('nonexistent-binary')}}]
E
t shell_empty r <<'E'
r:
  @echo [{{shell('')}}]
E
t uri r <<'E'
r:
  @echo {{encode_uri_component("a b/c?d=é")}}
E
t caps r <<'E'
r:
  @echo {{titlecase('a_b')}} {{capitalize('')}} {{kebabcase('fooBar')}} {{trim_start_match('aab','a')}} {{trim_end_matches('baa','a')}}
E
t error_fn_quoted r <<'E'
x := error('a')
r:
E
t nul r <<'E'
r:
  echo "{{"\u{0}"}}"
E
t env_nul r <<'E'
[env('A=B', 'x')]
r:
  @echo $A
E
