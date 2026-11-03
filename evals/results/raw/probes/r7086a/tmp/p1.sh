J=$(ls /target/debug/just 2>/dev/null || echo /work/crate/target/debug/just)
echo "bin $J"
T=/tmp/p; mkdir -p $T; cd $T
run() { echo "--- $1"; shift; "$@" 2>&1 | head -${LINES_MAX:-12}; echo "rc=${PIPESTATUS[0]}"; }
mk() { rm -rf $T/w; mkdir -p $T/w; cd $T/w; }

mk; printf 'x := datetime("%%Q")\nr:\n  echo {{x}}\n' > justfile; run datetime-bad $J r
mk; printf 'r:\n  echo {{ datetime_utc("%%") }}\n' > justfile; run datetime-utc-bad $J r

mk; mkdir -p a/b a/bar; printf 'mod foo "../bar"\nr:\n  echo\n' > a/b/justfile; echo 'x:' > a/bar/justfile; echo 'y:' > a/bar/.justfile; cd a/b; run ambiguous-mod-relative $J --list

mk; printf 'set unstable\nf(a) := f(a)\nr:\n  echo {{ f("x") }}\n' > justfile; run fn-recursion $J --unstable r
mk; printf 'x := f("a")\nf(a) := x\nr:\n  echo {{ x }}\n' > justfile; run fn-var-cycle $J --unstable r

mk; printf 'x := "orig"\nf(a) := a + x\nr:\n  echo {{ f("") }} {{x}}\n' > justfile; run fn-override $J --unstable x=new r
mk; printf 'x := `echo run >&2; echo v`\nf(a) := a + x\nr:\n  echo {{ f("") }} {{ f("") }} {{ x }}\n' > justfile; run fn-backtick-rerun $J --unstable r
mk; printf 'f(a) := is_dependency()\nd:\n  echo {{ f("") }} {{ is_dependency() }}\nr: d\n' > justfile; run fn-is-dep $J --unstable r

mk; printf 'mod a "x.just"\nmod b "x.just"\n' > justfile; printf 'v := "1"\nr:\n  echo {{v}} {{module_path()}} {{module_directory()}}\n' > x.just; run two-mods-a $J a::r; run two-mods-b $J b::r;  run two-mods-list $J --list; run two-mods-ov $J a::v=9 b::r
mk; printf 'mod a\nmod b\n' > justfile; printf 'import "common.just"\n' > a.just; printf 'import "common.just"\n' > b.just; printf 'v := "1"\nr:\n  echo {{v}} {{module_path()}}\n' > common.just; run shared-import-ov $J a::v=9 b::r; run shared-import-list $J --list

mk; printf 'set allow-duplicate-recipes\nset allow-duplicate-variables\nimport "a.just"\nimport "b.just"\n' > justfile; printf 'v := "a"\nr:\n  echo from-a {{v}}\n' > a.just; printf 'v := "b"\nr:\n  echo from-b {{v}}\n' > b.just;  run dup-import-order $J r

mk; printf 'set dotenv-path := "missing.env"\nr:\n  echo [$FOO]\n' > justfile; printf 'FOO=fromdotenv\n' > .env; run dotenv-path-missing $J r
mk; printf 'set dotenv-path := "missing.env"\nr:\n  echo [$FOO]\n' > justfile; run dotenv-path-missing2 $J r
mk; printf 'set dotenv-required\nset dotenv-path := "missing.env"\nr:\n  echo [$FOO]\n' > justfile; printf 'FOO=fromdotenv\n' > .env; run dotenv-path-missing-req $J r

mk; printf 'export FOO := "exported"\n[env("FOO","attr")]\nr:\n  echo $FOO\n[env("FOO","attr")]\ns:\n  #!/bin/sh\n  echo $FOO\n' > justfile; run env-vs-export $J r; run env-vs-export2 $J s
mk; printf 'set dotenv-load\n[env("FOO","attr")]\nr:\n  echo $FOO\n' > justfile; printf 'FOO=dot\n' > .env;  run env-vs-dotenv $J r

mk; printf 'x := "a!b$c\\\\d`e\\"f"\n' > justfile; $J --evaluate --evaluate-format shell > out.sh; cat out.sh;  run eval-shell sh -c '. ./out.sh; printf "%%s\n" "$x"'; run eval-bash bash -c '. ./out.sh; printf "%%s\n" "$x"'

mk; printf '[parallel]\na: b c\nb: x\nc: x\nx:\n  @echo running-x >&2; sleep 0.2; exit 3\n' > justfile; run parallel-fail-rerun $J a

mk; printf 'r:\n  echo hi\nx := "1"\n' > justfile; echo '--- stdout-full'; $J --list > /dev/full; echo rc=$?; $J --evaluate > /dev/full; echo rc=$?; $J --summary >/dev/full; echo rc=$?; $J --dump > /dev/full; echo rc=$?; echo '--- closed-pipe'; (sh -c "sleep 0.2; $J --list; echo rc=\$? >&2" | true); (sh -c "$J --evaluate x >&-; echo rc=\$? >&2")

mk; mkdir justfile; echo 'r:' > /dev/null; run justfile-dir $J --list
mk; mkdir sub; mkdir sub/justfile; printf 'r:\n  echo root\n' > justfile; cd sub; run justfile-dir-ancestor $J r
mk; mkdir d; printf 'r:\n  echo {{ which("d") }}|\n' > justfile; PATH=$PATH:$PWD run which-dir $J r
mk; printf 'r:\n  #!{{ "" }}\n  echo hi\n' > justfile; run shebang-empty $J r
mk; printf 'r:\n  #!/bin/sh\n  echo $0 {{ justfile() }}\n\n  echo line\n  exit 3\n' > justfile; run script-lines $J r
mk; printf '[script("sh")]\nr:\n  echo {{"a\nb"}}\n  echo c\n' > justfile; run script-attr-multiline $J -vvv r
