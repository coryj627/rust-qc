J=/target/debug/just
T=/tmp/p; mkdir -p $T; cd $T
run() { echo "--- $1"; shift; timeout 10 "$@" </dev/null 2>&1 | head -${LINES_MAX:-8}; echo "rc=${PIPESTATUS[0]}"; }
mk() { rm -rf $T/w; mkdir -p $T/w; cd $T/w; }
mk; printf 'trap "exit 1" TERM\nsleep 5 &\nwait\nexit 1\n' > g.sh; printf 'set guards\nr:\n  @?exec sh g.sh\n  @echo no\ns: r\n  @echo after-r\n' > justfile; ($J s </dev/null & P=$!; sleep 0.7; kill -TERM $P; wait $P; echo "guard-caught rc=$?")
mk; mkdir m; printf 'mod m\n_secret := "x"\nplain := "y"\n' > justfile; printf 'set export\nr:\n  @echo ${_secret-unset} ${plain-unset}\n' > m/mod.just; run mod-export-parent $J m::r
mk; mkdir m; printf 'set export\nmod m\nplain := "y"\n' > justfile; printf 'r:\n  @echo ${plain-unset}\n' > m/mod.just; run parent-export-mod $J m::r
mk; mkdir m; printf 'unexport FOO\nmod m\nr:\n  @echo ${FOO-unset}\n' > justfile; printf 'r:\n  @echo ${FOO-unset}\n' > m/mod.just; FOO=1 run unexport-root $J r; FOO=1 run unexport-mod $J m::r
mk; mkdir sub; printf '[working-directory("sub")]\nr:\n  @echo {{ `pwd` }} {{ path_exists("f") }} ; pwd\n' > justfile; touch sub/f; run wd-backtick $J r; grep -n "working-directory" /work/crate/README.md | head -8
