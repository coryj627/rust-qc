J=/target/debug/just
T=/tmp/p; mkdir -p $T; cd $T
run() { echo "--- $1"; shift; "$@" 2>&1 | head -${LINES_MAX:-12}; echo "rc=${PIPESTATUS[0]}"; }
mk() { rm -rf $T/w; mkdir -p $T/w; cd $T/w; }
mk; printf 'r:\n  echo hi\n' > justfile; run ts-bad $J --timestamp --timestamp-format '%%Q' r
cd /; printf 'r:\n  echo hi\n' > /justfile; run root-fmt $J --fmt --check;  run root-run $J r; rm -f /justfile
mk; printf 'r:\n  @ x\nx := \n' > justfile; run fmt-error-path $J --fmt; run normal-error-path $J --list
mk; printf 'x := shell("touch ran; echo v")\nr:\n  echo {{x}} {{ `touch ran2; echo w` }}\n' > justfile; run dry-shell $J --dry-run r; ls
mk; printf 'set guards\nr:\n  ?exit 3\n  echo no\n' > justfile; run guard-code $J r
mk; printf 'set guards\nr:\n  ?exit 1\n  echo no\ns: r\n  echo after-r\n' > justfile; run guard-1 $J s
run cmd-code $J --command sh -c 'exit 7'
mk; mkdir sub; printf 'p:\n  echo parent\n' > justfile; cd sub; run ceiling-rel $J --ceiling . p; run ceiling-abs $J --ceiling $PWD p; run ceiling-dotdot $J --ceiling ../sub p; JUST_CEILING=. run ceiling-env $J p; run ceiling-slash $J --ceiling $PWD/ p
mk; cat > justfile <<'J'
a := f'''
  one
  {{ "x\ny" }}
  two
'''
b := f"""
	t1
	t2
"""
c := """
	t1
	t2
"""
d := f'''{{ "" }}
  z
'''
e := f'''
    p
      q
  {{ "" }}r
'''
g := f'''
  k
'''
J
$J --evaluate a | od -c | head -5; echo; for v in b c d e g; do echo "[$v]"; $J --evaluate $v | od -c | head -4; done
mk; printf 'r:\n  echo a\\\\\n  echo b\n' > justfile; cat justfile; run dbl-backslash $J r
mk; printf 'r:\r\n  echo a \\\r\n    b\r\n  echo c\r\ns:\r\n  #!/bin/sh\r\n  echo d\r\nx := """\r\n  a\r\n  b\r\n"""\r\n' > justfile; run crlf $J r; run crlf2 $J s; $J --evaluate x | od -c | head -3; cp justfile o; run crlf-fmt $J --fmt --check; 
mk; printf 'mod? foo # c\nunexport FOO # c\nset export # c\nimport? "x" # c\nalias a := b # c\nb: # c\nx := "1" # c\n[private] # c\ny := "2"\n' > justfile; run comment-trailing $J --dump
mk; printf 'x := (\n  "a" +\n  "b"\n)\nset shell := [\n  "sh",\n  "-c",\n]\ny := if "a" == "b" {\n "c"\n} else {\n "d"\n}\nr:\n  echo {{\n  x\n }}\n' > justfile; run multi-line-fmt $J --dump; run multi-line-run $J r
mk; printf 'foo:\n  echo foo\n\n\n# trailing\n\n\nbar:\n\n\n  echo bar\n\n  echo baz\n\n\n' > justfile; run blank-lines $J --dump; run blank-run $J bar; 
mk; printf 'a := "1"\n\tb := "2"\n' > justfile; run indent-item $J --list
mk; printf '[linux]\nfoo:\n  echo l\n[macos]\nfoo:\n  echo m\n[windows]\nfoo:\n  echo w\nalias q := foo\n' > justfile; run os-dup $J q
mk; printf '[linux]\n[windows]\nfoo:\n  echo l\n[macos]\nfoo:\n  echo m\n[unix]\nfoo:\n  echo u\n' > justfile; run os-dup2 $J foo
mk; printf '[macos]\nfoo:\n  echo m\nbar: foo\n' > justfile; run disabled-dep $J bar; run disabled-show $J --show foo
mk; printf '[private]\nfoo:\n  echo p\nbar:\n' > justfile; run private-run $J foo; run private-show $J --show foo; run private-suggest $J fo; 
mk; printf '[group("a")]\n[group("b")]\nfoo:\n[group("b")]\nbar:\nbaz:\n' > justfile; run groups $J --list; run group-f $J --list --group b; run groups2 $J --groups; run summary $J --summary
mk; printf '# multi\n[doc("line1\\nline2")]\nfoo a b="x":\nverylongrecipename_verylongrecipename_verylongrecipename_abc x y z:\n# doc2\nb:\n[doc("é")]\nunicode_é:\n' > justfile; run list-doc $J --list
mk; mkdir m; printf 'mod m\n[group("g")]\nmod n "m/n.just"\nfoo:\n' > justfile; printf '# in m\nmr:\n' > m/mod.just; printf 'nr:\n' > m/n.just;  run list-sub $J --list --list-submodules; run list-sub2 $J --list m; run list-sub3 $J --list --unsorted;  run list-sub4 $J --list m::x; run list-group $J --list --group g; run list-group-sub $J --list --group g --list-submodules
