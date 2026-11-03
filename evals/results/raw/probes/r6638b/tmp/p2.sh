#!/bin/bash
J=/target/debug/just
export PATH=/target/debug:$PATH
f() { # name; stdin=justfile ; run --fmt twice and show result
  name=$1; shift
  d=$(mktemp -d /work/tmp/p/XXXXXX); cat > $d/justfile
  [ "$MK" ] && touch $d/$MK
  echo "=== FMT $name"
  (cd $d; cp justfile orig; timeout 20 $J --unstable --fmt "$@" 2>&1; echo "[exit $?]"; cat justfile; echo "--- diff-second-pass"; cp justfile one; timeout 20 $J --unstable --fmt 2>&1;  cmp one justfile )
}
f private_alias <<'E'
foo:

[private]
alias f := foo
E
f private_mod <<'E'
[private]
mod? sub
E
f doc_attr_mod <<'E'
[doc("line1\nline2")]
mod? sub
E
f doc_none_mod <<'E'
# a comment
[doc]
mod? sub
E
f doc_comment_mod <<'E'
# doc comment
mod? sub
E
f eager <<'E'
eager x := `echo hi`
E
f export_private <<'E'
[private]
export x := 'a'
E
f recipe_comment <<'E'
foo: # trailing recipe comment
  echo hi

# another

bar: # no body comment
baz: foo # c
  echo
E
f attr_comment <<'E'
[private] # attribute comment
foo:
  echo
E
f blank_lines <<'E'
foo:
  echo hi


bar:
  echo

baz:
quux:

x := 'a' # trailing
y := 'b'


# comment
z := 'c'
E
f sets <<'E'
set export := false
set quiet := true
set shell := ['bash', '-c']
set dotenv-load
set unstable := false
E
f format_strings <<'E'
x := f'a{{ "b" }}c'
y := f"a{{{{b}}c"
z := f'''
  foo{{x}}
'''
w := x'~/foo'
E
f expr <<'E'
a := if 'a' == 'b' { 'c' } else if 'd' != 'e' { 'f' } else { 'g' }
b := / 'x' / 'y'
c := ('a' + 'b') / 'c'
d := assert('a' == 'a', 'msg')
e := 'a' && 'b' || 'c'
f(x, y) := x + y
g() := 'a'
h := `echo hi`
i := ```
  echo hi
```
unexport FOO
E
f deps <<'E'
foo: (bar 'a' 'b') baz && (qux 'c') sub::x

@bar a b:
baz +x='1':
qux *$y:
E
f attrs <<'E'
[arg('a', long, short='b', help='h', pattern='p', value='v')]
[confirm('sure')]
[env('A', 'B')]
[extension('.sh')]
[group('g'), group("h")]
[metadata('m', 'n')]
[script('bash', '-x')]
[working-directory('w')]
foo a='':
  echo
E
f crlf < <(printf 'foo:\r\n  echo hi\r\n\r\nbar:\r\n  echo\r\n')
f import_mod <<'E'
import? 'x'
mod? a 'b'
mod? c
E
f shebang_recipe <<'E'
foo:
  #!/bin/sh

  echo {{ 'a' }}
    indented
  

  echo
E
f xstring_unset <<'E'
x := x'$UNSET_VAR_ZZ'
E
f comment_only <<'E'
# just a comment
E
f empty </dev/null
f mod_doc_before_group <<'E'
# doc
[group('g')]
mod? a
E
f bom < <(printf '\xef\xbb\xbfx := "a"\n')
