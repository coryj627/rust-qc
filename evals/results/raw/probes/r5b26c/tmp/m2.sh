cd /work/probe2; export CARGO_TARGET_DIR=/target/probe2
cp ../crate/Cargo.lock . 2>/dev/null
timeout 240 cargo test --offline --release --test p -- --nocapture --test-threads=1 mpmc_ arc_basic 2>&1 | grep -vE "^\s*(Compiling|Finished|Running)|BACKTRACE" | cut -c1-220 | tail -15
echo "=== miri pool"; MIRIFLAGS="-Zmiri-many-seeds=0..8 -Zmiri-permissive-provenance" timeout 240 cargo +nightly-2026-10-01 miri test --offline --test p pool_race 2>&1 | grep -E "^error|Undefined|test result|--> /work/crate|Data race" | cut -c1-260 | head -8
cd /work/probe; export CARGO_TARGET_DIR=/target/probe
echo "=== miri stale TB";  MIRIFLAGS="-Zmiri-tree-borrows" timeout 200 cargo +nightly-2026-10-01 miri test --offline --release --test soundness indexmap_retain_panic_stale 2>&1 | grep -E "^error|Undefined|test result|--> /work/crate" | cut -c1-260 | head -8
