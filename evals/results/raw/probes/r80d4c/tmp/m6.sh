cd /work/tmp/probe
cargo test --offline --test exercise -- --nocapture --test-threads=1 2>&1 | grep -E "^error|^test |panicked|assertion|left|right|hist from|string from" | cut -c1-200
echo "== exercise under miri"
MIRIFLAGS="-Zmiri-disable-isolation -Zmiri-permissive-provenance" timeout 280 cargo +nightly-2026-10-01 miri test --offline --test exercise -- --test-threads=1 2>&1 | grep -E "^error|^test |Undefined Behavior|-->|panicked|test result" | cut -c1-200 | head -20
echo "== baseline lib under miri"
cd /work/crate
MIRIFLAGS="-Zmiri-disable-isolation -Zmiri-permissive-provenance" timeout 500 cargo +nightly-2026-10-01 miri test --offline --lib --features=alloc,mpmc_large,portable-atomic-critical-section,serde,bytes 2>&1 | grep -E "^error|Undefined Behavior|panicked|test result|FAILED|^ *--> " | cut -c1-200 | head -20
