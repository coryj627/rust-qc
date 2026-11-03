J=/target/debug/just
T=/tmp/p; mkdir -p $T; cd $T
run() { echo "--- $1"; shift; timeout 10 "$@" </dev/null 2>&1 | head -${LINES_MAX:-8}; echo "rc=${PIPESTATUS[0]}"; }
mk() { rm -rf $T/w; mkdir -p $T/w; cd $T/w; }
mk; printf 'trap "exit 1" TERM\nsleep 5 &\nwait\nexit 1\n' > g.sh; printf 'set guards\nr:\n  @?sh g.sh\n  @echo no\ns: r\n  @echo after-r\n' > justfile; ($J s </dev/null & P=$!; sleep 0.7; kill -TERM $P; wait $P; echo "guard-caught rc=$?")
mk; printf 'r:\n  sh g.sh\n  echo no\n' > justfile; printf 'trap "exit 0" TERM\nsleep 5 &\nwait\nexit 0\n' > g.sh; ($J r </dev/null & P=$!; sleep 0.7; kill -TERM $P; wait $P; echo "nonguard-caught rc=$?")
mk; printf 'mod a "x.just"\nmod b "x.just"\n' > justfile; printf 'v := "1"\nr:\n  echo {{v}} {{module_path()}}\n' > x.just; run two-ov $J a::v=9 a::r; run two-ov2 $J b::v=9 a::r;  run two-sum $J --summary
