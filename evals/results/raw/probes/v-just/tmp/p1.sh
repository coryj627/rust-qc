#!/bin/bash
. /work/tmp/lib.sh
t "fmt eager"
printf 'set lazy\neager z := `touch ran; echo 3`\nr:\n  echo hi\n' > justfile; cp justfile orig
rr --unstable r; ls; rm -f ran
rr --unstable --fmt ; cat justfile; rr --unstable r; ls

t "fmt private alias/mod, doc"
printf 'foo:\n  echo hi\n\n[private]\nalias a := foo\n\n[private]\nmod m\n\n# c\n[doc]\nmod n "n.just"\n' > justfile; echo 'x:' > m.just; echo 'y:' > n.just
rr --list; rr --unstable --fmt; cat justfile; rr --list

t "fmt comments"
printf 'bar:\n  echo b\n\nqux: # comment before body\n  echo qux\n\nfoo: bar # depcomment\n  echo foo\n\n[private] # c\nbaz:\n  echo baz\n\nbodyless: # keep\n' > justfile
rr --unstable --fmt; cat justfile

t "fmt multi-line module doc"
printf '[doc("a\nb")]\nmod? m\n\nr:\n  echo\n' > justfile
rr --dump; rr --unstable --fmt; cat justfile; rr --list

t "shebang doc"
printf '#!/usr/bin/env just --justfile\nfoo:\n  echo hi\n' > justfile
rr --list; rr --dump; rr --unstable --fmt; cat justfile
t "aligned comment"
printf '#    aligned\nfoo:\n  echo hi\n' > justfile; rr --dump
t "private underscore assign"
printf "_x := 'v'\nfoo:\n  echo hi\n" > justfile; rr --dump

t "CRLF fmt"
printf 'a:\r\n  echo a\r\n\r\nb:\r\n  echo b\r\n' > justfile; rr --dump | od -c | head; rr --unstable --fmt; od -c justfile | head
t "CRLF string continuation"
printf 'x := "a\\\r\nb"\nr:\n  echo {{x}}\n' > justfile; rr r
printf 'x := "a\\\nb"\nr:\n  echo {{x}}\n' > justfile; rr r

t "fmt wrong root"
mkdir w; printf 'x := y\n' > w/justfile; rr --unstable --justfile w/justfile --fmt --check; cd w; rr --unstable --fmt --check; cd ..; printf 'x := \n' > w/justfile; rr --justfile w/justfile --list; rr --unstable --justfile w/justfile --fmt --check
ls -la /.dockerenv
rr --unstable --justfile /.dockerenv --fmt --check
rr --justfile /.dockerenv --list

t "recursive"
printf 'set unstable\nf(a) := f(a)\nx := f("a")\nr:\n  echo {{x}}\n' > justfile; rr r
printf 'set unstable\nf() := x\nx := f()\nr:\n  echo {{x}}\n' > justfile; rr r

t "fn context"
cat > justfile <<'E'
set unstable
x := "orig"
y := x
v := `echo run >&2; echo v`
export E := "exp"
f(a) := a + x
g(x) := y
h() := v
i() := `echo ${E-unset}`
j() := is_dependency()
r:
  echo {{f("")}} {{x}}
  echo {{g("param")}}
  echo {{h()}} {{h()}} {{v}}
  echo {{i()}} {{`echo ${E-unset}`}}
  echo {{j()}} {{is_dependency()}}
s: r
E
rr x=new r; rr s

t strftime
printf 'a := datetime("%%Q")\nr:\n  echo {{a}}\n' > justfile; rr r
printf 'a := datetime_utc("%%")\nr:\n  echo {{a}}\n' > justfile; rr r
printf 'r:\n  echo a\n' > justfile; rr --timestamp --timestamp-format '%Q' r

t "ambiguous module outside parent"
mkdir p bar; cd p; printf 'mod foo "../bar"\n' > justfile; touch ../bar/justfile ../bar/.justfile; rr --list
cd ..; rm ../$n/bar/.justfile; touch bar/mod.just; cd p; rr --list; 
t "ambig inside parent"
mkdir bar; printf 'mod foo "bar"\n' > justfile; touch bar/justfile bar/.justfile; rr --list

t "lexer backslash"
printf '\t\\' > justfile; rr --list
printf ' \\' > justfile; rr --list
printf 'x := "a"\n\t\\' > justfile; rr --list
printf 'a\n  \\' > justfile; rr --list
printf '\\' > justfile; rr --list

t "ignore comments"
cat > justfile <<'E'
set ignore-comments
r:
  #!/bin/sh
  # {{ nofn() }}
  echo a
E
rr r
sed -i 's/nofn()/nope/' justfile; rr r
sed -i 's/nope/uppercase()/' justfile; rr r
cat > justfile <<'E'
set ignore-comments
r:
  echo a \
  # {{ nofn() }}
E
rr r
cat > justfile <<'E'
set ignore-comments
set lazy
v := "x"
r:
  #!/bin/sh
  # {{v}}
  echo ok
E
rr r
