#!/bin/bash
J=/target/debug/just
export PATH=/target/debug:$PATH
t() { name=$1; shift
  d=$(mktemp -d /work/tmp/p/XXXXXX); cat > $d/justfile
  echo "=== $name :: just $*"
  (cd $d; timeout 20 $J "$@" 2>&1; echo "[exit $?]"; ls -A | grep -v justfile | sed 's/^/created: /')
}
t fallback_allow_missing --allow-missing nope <<'E'
set fallback
a:
E
t nofallback_allow_missing --allow-missing nope <<'E'
a:
E
t unexport_nonexport r <<'E'
foo := 'a'
unexport foo
r:
  @echo {{foo}}
E
d=$(mktemp -d /work/tmp/p/XXXXXX); mkdir -p $d/sub/tmpd; printf 'mod sub\n' > $d/justfile; printf 'set tempdir := "tmpd"\nr:\n  #!/bin/sh\n  echo "$0"\n' > $d/sub/mod.just; (cd $d; echo "== module tempdir"; $J sub r 2>&1; echo "[exit $?]")
d=$(mktemp -d /work/tmp/p/XXXXXX); mkdir -p $d/x; printf 'set working-directory := "x"\nset tempdir := "tmpd"\nr:\n  #!/bin/sh\n  echo "$0"\n' > $d/justfile; mkdir -p $d/x/tmpd; (cd $d; echo "== wd tempdir"; $J r 2>&1; echo "[exit $?]")
d=$(mktemp -d /work/tmp/p/XXXXXX); printf '[doc("l1\\nl2")]\nmod sub\nr:\n' > $d/justfile; printf 'q:\n' > $d/sub.just; (cd $d; echo "== mod multi doc list"; $J --list 2>&1; echo "[exit $?]")
d=$(mktemp -d /work/tmp/p/XXXXXX); printf 'r:\n  @echo "unstable=${JUST_UNSTABLE-no}"\nx := a() \n' > $d/justfile; printf 'a() := "1"\nr:\n  @echo {{a()}}\n' > $d/justfile; (cd $d; echo "== JUST_UNSTABLE=0";JUST_UNSTABLE=0 $J r 2>&1 | head -1 | cut -c1-90; echo "== JUST_UNSTABLE=1"; JUST_UNSTABLE=1 $J r 2>&1 | head -1; echo "== JUST_QUIET=1"; JUST_QUIET=1 $J r 2>&1 | head -1; echo "== JUST_YES=1";JUST_YES=1 $J r 2>&1 | head -1; echo '== JUST_YES=true';JUST_YES=true $J --unstable r 2>&1 | head -1)
d=$(mktemp -d /work/tmp/p/XXXXXX); printf 'set unstable\nmod sub\n' > $d/justfile; printf 'x := "a" || "b"\nr:\n  @echo {{x}}\n' > $d/sub.just; (cd $d; echo "== root set unstable, feature in module"; $J sub r 2>&1 | head -1 | cut -c1-100)
d=$(mktemp -d /work/tmp/p/XXXXXX); printf 'unexport FOO\nmod sub\nr:\n  @echo "root=${FOO-unset}"\n' > $d/justfile; printf 'r:\n  @echo "sub=${FOO-unset}"\n' > $d/sub.just; (cd $d; echo "== unexport inherit"; FOO=1 $J r sub r 2>&1)
