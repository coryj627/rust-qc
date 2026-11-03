J=/target/debug/just
mk() { d=$(mktemp -d -p /work/tmp/p); cd $d; cat > justfile; }
echo "--- 1 alias/dep to disabled recipe"
mk <<'J'
alias b := build
[windows]
build:
  echo
other:
  @echo other
J
$J other 2>&1 | head -2
mk <<'J'
[windows]
build:
  echo
all: build
other:
  @echo other
J
$J other 2>&1 | head -2
echo "--- 2 import? under a file"
mk <<'J'
import? 'f/x.just'
a:
  @echo a
J
echo hi > f; $J a 2>&1 | head -2
echo "--- 3 bash eval"
mk <<'J'
x := 'a!b'
J
which bash; bash -c 'eval "$('"$J"' --evaluate --evaluate-format shell)"; echo "bash x=[$x]"'
echo "--- 4 repro lines"
mk <<'J'
foo:
  @echo {{ 'a' }}
J
$J --fmt --check; echo rc=$?
echo "--- 5 --time and failure, --no-deps, --one default"
mk <<'J'
a: b
  @echo a
b:
  @echo b
J
$J --no-deps a; $J --one 2>&1; 
echo "--- 6 eager in module with set lazy off & fmt no"
mk <<'J'
set lazy
eager a := `echo ran-eager >&2; echo A`
b:
  @echo b
J
$J b 2>&1; cp justfile o; $J --fmt 2>/dev/null; $J b 2>&1; cat justfile
