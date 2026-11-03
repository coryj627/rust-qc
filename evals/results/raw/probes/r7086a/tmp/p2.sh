J=/target/debug/just
T=/tmp/p; mkdir -p $T; cd $T
run() { echo "--- $1"; shift; "$@" 2>&1 | head -${LINES_MAX:-14}; echo "rc=${PIPESTATUS[0]}"; }
mk() { rm -rf $T/w; mkdir -p $T/w; cd $T/w; }
mk; cat > justfile <<'J'
# leading comment
set unstable
set export := false

[private]
alias a := foo

[private]
x := "1" # trailing

_y := '2'

eager z := "3"

export w := f'a{{x}}b{{ x + "q" }}c'

[private]
[doc("modoc")]
[group("g")]
mod? m

[doc]
mod? n "n.just"

# docmod
mod? o

import? 'q.just'

unexport Q

f(a, b) := a + b

v := x'$HOME/~' + x"a" + '''
  ind
''' + """
  ind2\n
""" + ```
  echo
``` + if x == "1" { "a" } else if x =~ 'q' { "b" } else { "c" }

t := / 'a' / 'b' / ('c' && 'd' || 'e') + assert(x != "", 'bad') + f("a", 'b') + f'''
  fa{{x}}
  {{{{
'''

# doc comment
[group: 'g1']
[confirm("really")]
[env("A", "B")]
[arg('b', long='bee', short='B', pattern='p+', help='hh')]
[arg('c', long, value='vv')]
[metadata('k', 'v')]
[working-directory: 'sub']
@foo b='d' c=x $e=("a" + "b") *rest: bar (bar "x") && baz # depcomment
  echo {{b}} {{{{literal}}}}
  -@echo hi # shell-comment

  echo \
    cont
  # inner comment

# comment between
bar arg="":
baz: # trailing comment on bodyless

qux: # comment before body
  echo qux

[script('python3', '-c')]
[extension('.py')]
py:
  print(1)

sh:
  #!/usr/bin/env bash
  echo x
J
cp justfile orig
run dump0 sh -c "$J --unstable --dump > d1; $J --unstable --justfile d1 --working-directory . --dump > d2; cmp d1 d2; echo cmp=\$?"
run fmt $J --unstable --fmt
diff orig justfile
$J --unstable --dump --dump-format json > j1 2>/dev/null; cp orig justfile; $J --unstable --dump --dump-format json > j0; cmp j0 j1; echo jsoncmp=$?

mk; mkdir m; printf 'mod m\nalias x := m::foo\nfoo:\n  echo local\nbar:\n' > justfile; printf 'foo:\n  echo inmod\n' > m/mod.just; run alias-list $J --list; run alias-summary-run $J x

mk; printf 'a:\n  echo chosen-a\nb:\n  echo b\n' > justfile; run choose-empty $J --choose --chooser true

mk; printf 'mod a "x.just"\nmod b "x.just"\n' > justfile; printf 'r:\n  echo ran {{module_path()}}\n' > x.just; run two-mods-both $J a::r b::r

mk; printf 'x := "a!b$c\\\\d`e\\"f"\n' > justfile; $J --evaluate --evaluate-format shell > out.sh; cat out.sh; run eval-sh sh -c '. ./out.sh; printf "%s\n" "$x"'; run eval-bash bash -c '. ./out.sh; printf "%s\n" "$x"'

mk; printf 'set lazy\nx := `echo X >&2; echo x`\ny := `echo Y >&2; echo y`\nz := `echo Z >&2; echo z`\nf(a) := a + z\nr p=x:\n  echo {{p}} {{f("")}}\n[confirm(y)]\ns:\n  echo s\n' > justfile; run lazy1 $J --unstable --yes r; run lazy2 $J --unstable --yes s
mk; mkdir m; printf 'set lazy\nmod m\nd: m::r\n  echo d\nv := `exit 1`\n' > justfile; printf 'v := `echo V >&2; echo v`\nw := `exit 3`\nr:\n  echo {{v}}\n' > m/mod.just; run lazy-mod $J d
mk; mkdir m; printf 'mod m\nv := `echo rootv >&2; echo v`\n' > justfile; printf 'set lazy\nw := `echo modw >&2`\nr:\n  echo\n' > m/mod.just; run lazy-mod-setting $J m::r

mk; printf 'set positional-arguments\n[arg("o", long)]\nr o="dflt" p="":\n  echo "1=$1 2=${2:-} n=$#"\n' > justfile; run pos-opt $J r P; run pos-opt2 $J r --o O P
mk; printf 'set guards\nr:\n  ?false\n  echo notreached\ns:\n  ?-false\n' > justfile; run guard $J r; run guard2 $J s
mk; printf 'r:\n  @{{""}}echo x\n  {{"@"}}echo y\n  {{"-"}}false\n  echo z\n' > justfile; run sigil-interp $J r
mk; printf 'r:\n  {{""}}@echo x\n' > justfile; run sigil-interp2 $J r
mk; printf 'r:\n  -{{""}}\n  @\n  echo z\n' > justfile; run sigil-only $J r
mk; printf 'set ignore-comments\nr:\n  # c \\\n  echo after\n  echo x \\\n  # y\n  echo z\n' > justfile; run ignore-comments $J r
mk; printf 'r:\n  echo a\\\n\n  echo b\n' > justfile; run cont-blank $J r
mk; printf 'r:\n  echo a \\' > justfile; run cont-eof $J r
mk; printf 'r:\n  echo "é" \\\n     "ü"\n  false\n' > justfile; run lineno $J r
mk; printf 'x := "é"\nr: (nope x)\n' > justfile; run err-unicode $J r
mk; printf 'x := "abc\n' > justfile; run unterminated $J --list
mk; printf 'x := "abc' > justfile; run unterminated2 $J --list
mk; printf 'x := "a"\n\n\n' >justfile;  printf 'r:\n\tfoo\n  bar\n' > justfile; run mixed-indent $J --list
mk; printf 'a:\n  echo a\n' > justfile; for a in "" "a:::b" ":" "::" "a::" "::a"; do run "show[$a]" $J --show "$a"; run "usage[$a]" $J --usage "$a"; run "list[$a]" $J --list "$a"; run "eval[$a]" $J --evaluate "$a"; run "run[$a]" $J "$a"; done
run set1 $J --set "" x; run set2 $J --set a::b x; run set3 $J "=x" ; run set4 $J --evaluate "a=b"; run set5 $J "a::=x"; run cmd $J --command nonexistent-binary-q; run cmd2 $J --command false
