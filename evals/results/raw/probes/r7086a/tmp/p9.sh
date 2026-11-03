J=/target/debug/just
T=/tmp/p; mkdir -p $T; cd $T
run() { echo "--- $1"; shift; timeout 10 "$@" </dev/null 2>&1 | head -${LINES_MAX:-10}; echo "rc=${PIPESTATUS[0]}"; }
mk() { rm -rf $T/w; mkdir -p $T/w; cd $T/w; }
ls -a / | head -30 | tr '\n' ' '; echo
F=$(find / -maxdepth 1 -type f -readable 2>/dev/null | head -1); echo "rootfile=$F"; if [ -n "$F" ]; then cd /tmp; run root-file-fmt $J --justfile $F --fmt --check; run root-file-list $J --justfile $F --list; fi
mk; printf 'set lazy\nx := assert("a" == "b", "unused assert")\ny := if "a" =~ "(" { "c" } else { "d" }\nr:\n  echo r\n' > justfile; run lazy-assert $J r
mk; printf 'x := assert("a" == "b", "unused assert")\nr:\n  echo r\n' > justfile; run assert-list $J --list; run assert-dump $J --dump; run assert-fmt $J --fmt --check
mk; printf 'b x:\n  echo b-got {{x}}\na: (b is_dependency())\n  echo a {{ is_dependency() }}\n[confirm(is_dependency())]\nc:\n  echo c\n' > justfile; run isdep-depargs $J a; echo n | timeout 5 $J c; echo rc=$?
mk; printf 'set guards\nr:\n  ?sh -c "trap \\"exit 1\\" TERM; sleep 5 & wait"\n  echo no\ns: r\n  echo after-r\n' > justfile; ($J s </dev/null & P=$!; sleep 0.7; kill -TERM $P; wait $P; echo "guard-caught rc=$?")
mk; printf 'r:\n  echo {{\n    "a"\n  }}\n  false\n' > justfile; run line-multi $J r
mk; printf 'r:\n  echo {{ """\n    a\n  """ }}\n  false\n' > justfile; run line-multi2 $J r
mk; printf '# c\n[private]\nr:\n\n  # cmt\n\n  echo x\n  false\n' > justfile; run line-simple $J r
mk; mkdir p q; printf '[private]\nmod p\nmod q\nz:\n' > justfile; printf 'pr:\n  echo private-module-recipe\n' > p/mod.just; printf 'qr:\n' > q/mod.just; run choose-private $J --choose --chooser 'cat; echo z'; run summary-private $J --summary; run list-private $J --list
mk; python3 -c "
for i in range(20000): print(f'r{i}:')" > justfile; echo '--- list-head'; $J --list 2>err | head -1; echo "rc=${PIPESTATUS[0]}"; cut -c1-120 err | head -3
mk; printf 'x := "1"\nfoo:\n' > justfile; run evaluate-mod-nomatch $J --evaluate nope; run evaluate-fn $J --evaluate foo
mk; mkdir m; printf 'mod m\nx := "r"\n' > justfile; printf 'y := "m"\n_z := "p"\n' > m/mod.just; run eval-mod $J --evaluate m; run eval-mod2 $J --evaluate m::y; run eval-mod3 $J --evaluate m y; run eval-mod4 $J --evaluate m::_z
mk; printf 'foo-bar := "1"\n_h := "2"\nexport e := "3"\n' > justfile; run eval-sh $J --evaluate --evaluate-format shell; 
mk; printf 'set unstable\nx := "a"\nf(x) := x + "b"\nr:\n  echo {{ f("p") }} {{x}}\n' > justfile; run fn-shadow-var $J r
mk; printf 'set unstable\nf(a, b="d") := a\n' > justfile; run fn-default $J --list
mk; printf 'set unstable\nf() := g()\ng() := f()\nx := "1"\nr:\n  echo {{ x }}\n' > justfile; run fn-mutual-unused $J r
