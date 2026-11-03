J=/target/debug/just
T=/tmp/p; mkdir -p $T; cd $T
run() { echo "--- $1"; shift; timeout 10 "$@" </dev/null 2>&1 | head -${LINES_MAX:-10}; echo "rc=${PIPESTATUS[0]}"; }
mk() { rm -rf $T/w; mkdir -p $T/w; cd $T/w; }
# ignore-comments + interpolations
mk; printf 'set ignore-comments\nr:\n  #!/bin/sh\n  # {{nope}}\n  echo hi\n' > justfile; run ic-script-var $J r
mk; printf 'set ignore-comments\nr:\n  #!/bin/sh\n  # {{ nofn() }}\n  echo hi\n' > justfile; run ic-script-fn $J r
mk; printf 'set ignore-comments\nr:\n  echo a \\\n  # {{nope}}\n  echo b\n' > justfile; run ic-cont-var $J r
mk; printf 'set ignore-comments\nr:\n  echo a \\\n  # {{nofn()}}\n  echo b\n' > justfile; run ic-cont-fn $J r
mk; printf 'set ignore-comments\nr:\n  # {{ nofn() }}\n  # {{ nope }}\n  # {{ uppercase() }}\n  echo b\n' > justfile; run ic-lines $J r
mk; printf 'set ignore-comments\nset lazy\ni := "/bin/sh"\nr:\n  #!{{i}}\n  echo hi\n' > justfile; run ic-lazy-shebang $J r
mk; printf 'set ignore-comments\ni := "/bin/sh"\nr:\n  #!{{i}}\n  echo hi\n' > justfile; run ic-shebang $J r
mk; printf 'set ignore-comments\nset lazy\nv := "x"\n[script("sh")]\nr:\n  # {{v}}\n  echo hi\n' > justfile; run ic-lazy-script $J r
# mod with file of same stem
mk; printf 'mod build\nr:\n' > justfile; printf 'x:\n  echo in-build\n' > build.just; printf '#!/bin/sh\n' > build; run mod-file-stem $J build::x
mk; printf 'mod? build\nr:\n  echo r\n' > justfile; printf 'bin' > build; run mod-opt-file $J r
mk; printf 'mod build\nr:\n' > justfile; mkdir build; mkdir build/justfile; run mod-dir-named-justfile $J --list
mk; printf 'mod? a "x/y"\nr:\n  echo r\n' > justfile; touch x; run mod-opt-notdir $J r
mk; printf 'import? "x/y"\nr:\n  echo r\n' > justfile; touch x; run import-opt-notdir $J r
mk; printf 'import? "nonexistent/y"\nr:\n  echo r\n' > justfile; run import-opt-missing $J r
# init
mk; touch .justfile; run init-dotjustfile $J --init; ls -a; run after-init $J --list
mk; touch Justfile; run init-case $J --init; ls -a; run after-init2 $J --list
# module invocations
mk; mkdir m; printf 'mod m\nfoo:\n  echo foo\n' > justfile; printf 'default a="1":\n  echo d {{a}}\nother:\n  echo other\n' > m/mod.just; run m0 $J m; run m1 $J m other; run m2 $J m::other; run m3 $J m:: ; run m4 $J m nope; run m5 $J m::other::x; run m6 $J foo m; run m7 $J m foo;
# --fmt idempotence / write
mk; printf 'x:=1\n' > justfile; run fmt-err $J --fmt
mk; printf 'x := "1"\n' > justfile; chmod 444 justfile; run fmt-ro $J --fmt
# editor signal
mk; printf 'r:\n' > justfile; printf '#!/bin/sh\nsleep 3; echo editor-done\n' > /work/tmp/ed; chmod +x /work/tmp/ed; (VISUAL=/work/tmp/ed $J --edit & P=$!; sleep 0.5; kill -INT $P; sleep 0.2; if kill -0 $P 2>/dev/null; then echo just-still-alive; else echo just-exited; fi; wait $P; echo "rc=$?"; pgrep -f /work/tmp/ed | wc -l)
VISUAL="sh -c true" run visual-args $J --edit
# chroot root justfile
mkdir -p /work/tmp/rootfs/lib /work/tmp/rootfs/lib64 /work/tmp/rootfs/usr/lib /work/tmp/rootfs/bin; cp /target/debug/just /work/tmp/rootfs/bin/just; for l in $(ldd /target/debug/just | grep -o '/[^ ]*'); do mkdir -p /work/tmp/rootfs$(dirname $l); cp -L $l /work/tmp/rootfs$l; done; printf 'r:\n    echo hi\n' > /work/tmp/rootfs/justfile
id -u; chroot /work/tmp/rootfs /bin/just --fmt --check 2>&1 | head -5; echo "chroot rc=${PIPESTATUS[0]}"; chroot /work/tmp/rootfs /bin/just --list 2>&1 | head -5; echo rc=$?
unshare -r chroot /work/tmp/rootfs /bin/just --fmt --check 2>&1 | head -5; echo "unshare rc=${PIPESTATUS[0]}"
