export RUST_BACKTRACE=0
for t in "$@"; do echo "=== $t"; timeout 120 /target/probe/debug/p2 $t 2>&1 | grep -v 'note: run with\|^$' | cut -c1-600; echo "exit=${PIPESTATUS[0]}"; done
