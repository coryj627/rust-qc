cd /work/probe; export CARGO_TARGET_DIR=/target/probe
cargo test --offline --release --test bign n_131072 -- --nocapture 2>&1 | grep -E "len=|wrong|dup|panicked" | cut -c1-200
cargo test --offline --release --test soundness indexmap_retain_panic_stale -- --nocapture 2>&1 | grep -E "len=|removed|panicked|double|free|signal|result" | cut -c1-200
for t in peek_race ready_dequeue_unchecked_race ready_enqueue_unchecked_race; do
 echo "=== miri $t"; MIRIFLAGS="-Zmiri-many-seeds=0..16" timeout 280 cargo +nightly-2026-10-01 miri test --offline --test spsc_race $t 2>&1 | grep -E "^error|Data race|test result|exit|Undefined|^\s+--> " | cut -c1-260 | head -8
done
echo "=== miri stale"; timeout 200 cargo +nightly-2026-10-01 miri test --offline --release --test soundness indexmap_retain_panic_stale 2>&1 | grep -E "^error|Undefined|test result|--> /work/crate" | cut -c1-260 | head -8
