cd /work/tmp/pm
echo "--- neon"; cargo +nightly-2026-10-01 miri run --offline 2>&1 | grep -v -E "^warning|^ +(-->|\||=)|^ *[0-9]+ \||^$|Compiling|Finished|Running" | head -30
cd /work/tmp/pm2
for c in strs; do echo "--- SB $c"; cargo +nightly-2026-10-01 miri run --offline -- $c 2>&1 | grep -E "Undefined|error|alignment|^\[|ok|tag|stage2|lib.rs" | head -14; done
echo "--- TB strs"; MIRIFLAGS=-Zmiri-tree-borrows cargo +nightly-2026-10-01 miri run --offline -- strs 2>&1 | grep -E "Undefined|error:|^\[|Some" | head
for c in dbl reset lazy; do echo "--- release TB $c"; MIRIFLAGS=-Zmiri-tree-borrows cargo +nightly-2026-10-01 miri run --release --offline -- $c 2>&1 | grep -E "Undefined|error:|panicked|safer_unch|Array|-->" | head -8; done
for c in dup big; do echo "--- TB $c"; MIRIFLAGS="-Zmiri-tree-borrows -Zmiri-disable-isolation" cargo +nightly-2026-10-01 miri run --offline -- $c 2>&1 | grep -E "Undefined|error:|panicked|^[0-9]+" | head -8; done
cd /work/tmp/m2
echo "--- with_capacity(0)"; MIRIFLAGS=-Zmiri-tree-borrows cargo +nightly-2026-10-01 miri test --offline --lib test_send_sync 2>&1 | grep -E "Undefined|error:|test result|size 0" | head -5
