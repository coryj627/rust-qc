#!/bin/bash
J=/target/debug/just
export PATH=/target/debug:$PATH
t() { name=$1; shift
  d=$(mktemp -d /work/tmp/p/XXXXXX); cat > $d/justfile
  echo "=== $name :: just $*"
  (cd $d; timeout 20 $J "$@" 2>&1; echo "[exit $?]"; ls -A | grep -v justfile | sed 's/^/created: /')
}
# shared import across modules
d=$(mktemp -d /work/tmp/p/XXXXXX); printf 'mod a\nmod b\n' > $d/justfile; printf 'import "common.just"\n' > $d/a.just; printf 'import "common.just"\n' > $d/b.just; printf 'x := "orig"\nr:\n  @echo x={{x}} mod={{module_path()}}\n' > $d/common.just; (cd $d; echo "== shared import"; $J a r b r 2>&1; $J a::x=over a r b r 2>&1; echo "[exit $?]")
# same module file twice
d=$(mktemp -d /work/tmp/p/XXXXXX); printf 'mod a "x.just"\nmod b "x.just"\n' > $d/justfile; printf 'y := "v"\nr:\n  @echo mod={{module_path()}} y={{y}}\n' > $d/x.just; (cd $d; echo "== shared module"; $J a r 2>&1; echo "[exit $?]"; $J b r 2>&1; echo "[exit $?]"; $J a::y=1 b r 2>&1; echo "[exit $?]"; $J --summary 2>&1; $J --dump --dump-format json | python3 -c 'import json,sys; d=json.load(sys.stdin); print({k:(v["module_path"], v["recipes"]["r"]["namepath"]) for k,v in d["modules"].items()})')
# same file as module in different dirs with relative paths
t dry_shell -n a <<'E'
x := shell('touch marker; echo v')
a:
  echo {{x}} {{`touch marker2; echo bt`}}
E
for v in JUST_LIST_SUBMODULES=1 JUST_GROUP=g JUST_WORKING_DIRECTORY=/tmp JUST_CHECK=1; do d=$(mktemp -d /work/tmp/p/XXXXXX); printf 'a:\n  @echo ok\n' > $d/justfile; (cd $d; echo "== env $v"; env $v $J a 2>&1 | head -5; echo "[exit ${PIPESTATUS[0]}]"); done
d=$(mktemp -d /work/tmp/p/XXXXXX); printf 'a:\n  echo ok\n' > $d/justfile; (cd $d; echo "== JUST_VERBOSE=0"; JUST_VERBOSE=0 $J a 2>&1; echo "== JUST_QUIET=0"; JUST_QUIET=0 $J a 2>&1; echo "== JUST_DRY_RUN=false";JUST_DRY_RUN=false $J a 2>&1; echo "[exit $?]")
d=$(mktemp -d /work/tmp/p/XXXXXX); mkdir $d/w; printf 'set working-directory := "w"\nset dotenv-load\nexport E := "1"\na:\n  @pwd\n' > $d/justfile; echo 'D=fromroot' > $d/.env; echo 'D=fromw' > $d/w/.env; (cd $d; echo "== --command wd"; $J --command pwd 2>&1; $J --command sh -c 'echo E=$E D=$D' 2>&1; echo "[exit $?]")
t alias_disabled a <<'E'
[windows]
foo:
  echo win
alias f := foo
a:
  echo a
E
d=$(mktemp -d /work/tmp/p/XXXXXX); printf 'a:\n' > $d/justfile; (cd $d; echo "== editor w/ args"; EDITOR='true --wait' VISUAL= $J --edit 2>&1; echo "[exit $?]"; env -u VISUAL EDITOR='true --wait' $J --edit 2>&1; echo "[exit $?]")
d=$(mktemp -d /work/tmp/p/XXXXXX); printf 'secret := "s"\nmod sub\n' > $d/justfile; printf 'set export\nr:\n  @echo "secret=${secret-unset}"\n' > $d/sub.just; (cd $d; echo "== parent var leak"; $J sub r 2>&1; echo "[exit $?]")
d=$(mktemp -d /work/tmp/p/XXXXXX); printf 'a:\n  echo hi\n' > $d/justfile; (cd $d; echo "== complete --"; JUST_COMPLETE=bash $J -- 2>&1 | tail -3; echo "[exit ${PIPESTATUS[0]}]"; echo '== fmt indentation empty'; $J --fmt --indentation '' 2>&1; echo "[exit $?]"; cat justfile;  $J a 2>&1; echo "[exit $?]"; printf 'a:\n  echo hi\n' > justfile; echo '== fmt indentation x';  $J --fmt --indentation 'xx' 2>&1; echo "[exit $?]"; cat justfile)
t fstr_unindent a <<'E'
v := "A\nB"
x := f'''
    one{{v}}
    two
  '''
y := """
    one\n  two
  """
z := f"""
    one\n  two
  """
a:
  @echo '[{{x}}]' ; echo '[{{y}}]'; echo '[{{z}}]'
E
t backslash_eof a < <(printf "x := 'a' \\\\")
t backslash_eof2 a < <(printf "a:\n  echo hi\nx := 'a' \\\\")
t mod_eval sub <<'E'
E
t crlf_cont a < <(printf 'x := "a\\\r\nb"\r\na:\r\n  @echo {{x}}\r\n')
t guard_sigil a <<'E'
set guards
a:
  ?false
  echo after
E
t guard_quiet a <<'E'
set guards
a:
  @?true
  ?@true
  echo after
E
t guard_unset a <<'E'
a:
  ?true
E
t sigil_interp a <<'E'
x := '-echo'
a:
  {{x}} hi
E
t sigil_cont a <<'E'
a:
  @echo a \
    b
  -false
  @-echo \
  @c
E
t empty_after_sigil a <<'E'
a:
  @
  -
  echo ok
E
t show_doc_multi --list <<'E'
[doc("x\ny")]
a:
# comment `code` here
b:
E
t list_long --list <<'E'
# doc
averyveryveryveryveryveryveryveryveryveryveryveryveryveryverylongname:
# doc
b:
E
