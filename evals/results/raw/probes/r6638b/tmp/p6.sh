#!/bin/bash
J=/target/debug/just
export PATH=/target/debug:$PATH
t() { name=$1; shift
  d=$(mktemp -d /work/tmp/p/XXXXXX); cat > $d/justfile
  echo "=== $name :: just $*"
  (cd $d; timeout 20 $J "$@" 2>&1; echo "[exit $?]"; ls -A | grep -v justfile | sed 's/^/created: /')
}
t assert_list --list <<'E'
x := assert('a' == 'b', 'nope')
r:
E
t assert_lazy r <<'E'
set lazy
x := assert('a' == 'b', 'nope')
r:
  @echo ran
E
t assert_override x=1 r <<'E'
x := assert('a' == 'b', 'nope')
r:
  @echo ran {{x}}
E
t error_lazy r <<'E'
set lazy
x := error('nope')
r:
  @echo ran
E
t no_quiet_shebang r <<'E'
set quiet
[no-quiet]
r:
  #!/bin/sh
  echo body
E
t no_quiet_line r <<'E'
set quiet
[no-quiet]
r:
  echo body
E
t no_quiet_at r <<'E'
set quiet
[no-quiet]
@r:
  echo body
E
t set_quiet_at_shebang r <<'E'
set quiet
@r:
  #!/bin/sh
  echo body
E
t missing_wd_attr r <<'E'
[working-directory('nonexistent')]
r:
  echo hi
E
t missing_wd_set r <<'E'
set working-directory := 'nonexistent'
r:
  echo hi
E
t missing_wd_script r <<'E'
[working-directory('nonexistent')]
r:
  #!/bin/sh
  echo hi
E
t missing_wd_backtick r <<'E'
set working-directory := 'nonexistent'
x := `echo hi`
r:
  echo {{x}}
E
t env_param_shell r <<'E'
[env('FOO', 'attr')]
r x=shell('echo ${FOO-unset}') y=`echo ${FOO-unset}`:
  @echo x={{x}} y={{y}} body={{`echo ${FOO-unset}`}} run=$FOO
E
d=$(mktemp -d /work/tmp/p/XXXXXX); mkdir -p $d/dir; printf 'mod a "../dir/justfile"\n' > $d/dir/justfile; (cd $d/dir; echo "== circular mod via .."; timeout 30 $J --list 2>&1 | cut -c1-200 | head -4; echo "[exit ${PIPESTATUS[0]}]")
d=$(mktemp -d /work/tmp/p/XXXXXX); mkdir -p $d/dir; printf 'import "../dir/justfile"\nr:\n' > $d/dir/justfile; (cd $d/dir; echo "== circular import via .."; timeout 30 $J --list 2>&1 | cut -c1-200 | head -4; echo "[exit ${PIPESTATUS[0]}]")
d=$(mktemp -d /work/tmp/p/XXXXXX); mkdir -p $d/bin; ln -s /bin/sh $d/bin/pwsh; printf "[script('$(echo $d)/bin/pwsh')]\na:\n  echo \$0 | sed 's/.*\\\\///'\n[script('pwsh')]\nb:\n  echo \$0 | sed 's/.*\\\\///'\n" > $d/justfile; cat $d/justfile; (cd $d; echo "== pwsh ext"; PATH=$d/bin:$PATH $J a b 2>&1; echo "[exit $?]")
# parallel ETXTBSY
d=$(mktemp -d /work/tmp/p/XXXXXX); python3 - > $d/justfile <<'P'
n=150
print("[parallel]\nall: "+" ".join(f"s{i}" for i in range(n)))
for i in range(n): print(f"s{i}:\n  #!/bin/sh\n  true\n")
P
(cd $d; echo "== parallel scripts"; for k in 1 2 3 4 5 6 7 8; do timeout 60 $J all 2>&1 | grep -i -m2 'busy\|error' ; echo "run $k exit ${PIPESTATUS[0]}"; done)
