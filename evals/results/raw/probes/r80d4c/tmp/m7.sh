cd /work/tmp/probe
MIRIFLAGS="-Zmiri-disable-isolation -Zmiri-many-seeds=0..16 -Zmiri-ignore-leaks" timeout 200 cargo +nightly-2026-10-01 miri test --offline --test ub -- --exact spsc_producer_ready_race2 2>&1 | grep -E "Data race|crate/src/spsc" | cut -c1-200 | sort | uniq -c | head -5
cd /work/crate
echo "== docsrs"; RUSTDOCFLAGS="--cfg docsrs" timeout 200 cargo +nightly-2026-10-01 doc --offline --no-deps --features=alloc,mpmc_large,portable-atomic-critical-section,serde,ufmt,bytes,defmt --target-dir /target/ds 2>&1 | grep -E "^error|E0557|Finished|removed" | cut -c1-160 | head -5
rustc --print target-list | grep -E '^arm64' | tr '\n' ' '; echo
