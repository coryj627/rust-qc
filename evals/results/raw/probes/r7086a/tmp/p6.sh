J=/target/debug/just
T=/tmp/p; mkdir -p $T; cd $T
run() { echo "--- $1"; shift; "$@" 2>&1 | head -${LINES_MAX:-12}; echo "rc=${PIPESTATUS[0]}"; }
mk() { rm -rf $T/w; mkdir -p $T/w; cd $T/w; }
mk; printf '\t\\' > justfile; run tab-backslash $J --list
mk; printf 'x := "a"\n\t\\' > justfile; run tab-backslash2 $J --list
mk; printf 'foo:\n  echo\n\t\\' > justfile; run tab-backslash3 $J foo
mk; printf ' \\\n' > justfile; run sp-bs $J --list
mk; printf '\t\\\n' > justfile; RUST_BACKTRACE=1 LINES_MAX=30 run tab-bs4 $J --list
mk; printf '\t#\n' > justfile; run tab-comment $J --list
mk; printf '\t\\x' > justfile; run tab-bsx $J --list
mk; printf 'a := (\n\t\\\n)\n' > justfile; run delim-bs $J --list
mk; printf '[doc("a\\nb")]\nmod? m\n' > justfile; run mod-doc-multiline $J --dump; $J --dump > d; cp d justfile; run mod-doc-reparse $J --list
mk; printf '#!/usr/bin/env just --justfile\nfoo:\n  echo hi\n' > justfile; run shebang-doc $J --list; run shebang-fmt $J --dump
mk; printf '[arg("o", long)]\nr o="d" p:\n  echo {{o}} {{p}}\n' > justfile; run opt-default-then-req $J r x
mk; printf 'a:\n  b\n \tc\n' > justfile; run mixed-body $J --list
mk; printf 'x := "é" + nope\n' > justfile; run col-unicode $J --list
mk; printf 'x := "日本語日本語" + nope\n' > justfile; run col-cjk $J --list
