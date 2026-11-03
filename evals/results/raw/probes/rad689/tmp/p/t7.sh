J=/target/debug/just
mk() { d=$(mktemp -d -p /work/tmp/p); cd $d; cat > justfile; }
echo "--- 1 env requires"
mk <<'J'
a:
  @echo a
[group('g')]
b:
  @echo b
J
JUST_LIST_SUBMODULES=true $J a 2>&1 | head -3; JUST_GROUP=g $J a 2>&1 | head -3; JUST_GROUP=g $J --list 2>&1 | head -5
echo "--- 2 indentation fmt"
$J --dump --indentation '' 2>&1 | head -4; $J --dump --indentation 'xx' 2>&1 | head -4; $J --dump --indentation "$(printf ' \t')" 2>&1 | head -4 | od -c | head -3
echo "--- 3 sigpipe"
python3 - <<'P'
s=''.join(f"recipe{i}:\n  @echo x\n" for i in range(3000))
open('big.just','w').write(s)
P
for flag in --list --summary --dump --variables --json --groups; do  $J -f big.just $flag 2>err | (exec 0<&-; sleep 0.3); echo "$flag status: $(cat err | head -2)"; done
$J -f big.just --list 2>&1 >/dev/full | head -2; echo "full rc=$?"
echo "--- 4 choose with default param"
mk <<'J'
a x='d':
  @echo a x={{x}}
b:
  @echo b
[private]
c:
  @echo c
J
$J --choose --chooser 'cat >/dev/null; printf "a\nb\n"' 2>&1
echo "--- 5 show dep in module"
mk <<'J'
mod m
a: m::bar (m::baz 'x')
J
printf 'bar:\nbaz y:\n' > m.just
$J --show a 2>&1; $J --dump 2>&1; $J --json | python3 -c "import json,sys; print(json.load(sys.stdin)['recipes']['a']['dependencies'])"
echo "--- 6 missing wd message"
mk <<'J'
set working-directory := 'nope'
a:
  @echo hi
[working-directory('nope2')]
b:
  @echo hi
c:
  #!/bin/sh
  echo
J
$J a 2>&1 | head -2; $J b 2>&1 | head -2; $J c 2>&1 | head -3
echo "--- 7 shell missing"
$J --shell nosuchshell a 2>&1 | head -2
echo "--- 8 --fmt at root"
ls -ld /; printf 'a:\n' > /justfile 2>/dev/null && (cd /; $J --fmt 2>&1 | head -3; rm -f /justfile) || echo cannot-write-root
echo "--- 9 bom"
printf '\xef\xbb\xbfa:\n  @echo bom\n' > justfile; $J a 2>&1; $J --dump | od -c | head -1
printf '\xef\xbb\xbf  a:\n' > justfile; $J --dump 2>&1 | head -3
printf '\xef\xbb\xbf' > justfile; $J --dump 2>&1 | head -3; printf '\xef\xbb\xbf\n\xef\xbb\xbf' > justfile; $J --dump 2>&1 | head -5
echo "--- 10 lone CR and others"
printf 'a:\r  echo\n' > justfile; $J a 2>&1 | head -4
printf 'a:\n  echo x\r\n  echo y\r' > justfile; $J a 2>&1 | od -c | head -3
printf 'x := "a\rb"\na:\n  @echo {{x}}\n' > justfile; $J a 2>&1 | od -c | head -2
printf "x := '''\r\n  a\r\n  b\r\n'''\na:\n  @printf '%%s' '{{x}}'\n" > justfile; $J a 2>&1 | od -c | head -3
printf "# comment\r\nfoo:\r\n  @echo 'a'  \\\\\r\n   b\r\n" > justfile; $J foo 2>&1 | od -c | head -3
echo "--- 11 mod file ambiguous abs path, dot"
d=$(mktemp -d -p /work/tmp/p); mkdir -p $d/o; printf 'a:\n' > $d/o/mod.just;  printf 'a:\n' > $d/o/justfile; cd $d; printf "mod foo '$d/o'\n" > justfile;$J --list 2>&1 | head -3;  printf "mod foo 'o'\n" > justfile; $J --list 2>&1 | head -3
printf "mod foo 'o/'\n" > justfile; $J --list 2>&1 | head -3;  printf "mod foo './o'\n" > justfile; $J --list 2>&1 | head -3
