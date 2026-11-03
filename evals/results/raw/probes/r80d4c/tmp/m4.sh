cd /work/tmp/probe
cargo test --offline --test more -- --nocapture --test-threads=1 truncate_with_default_hasher 2>&1 | grep -E "^error|^test |panicked|lost-key|get=|reinsert|left|right" | cut -c1-240
echo "== pool race detail"
MIRIFLAGS="-Zmiri-disable-isolation -Zmiri-many-seeds=0..24 -Zmiri-tree-borrows -Zmiri-ignore-leaks -Zmiri-permissive-provenance" timeout 200 cargo +nightly-2026-10-01 miri test --offline --test ub -- --exact pool_race 2>&1 | grep -E "Data race|crate/src/pool|occurred|boxed.rs|cas.rs|treiber" | cut -c1-200 | head -14
echo "== mpmc stress"
cargo test --offline --release --test more -- --nocapture --test-threads=1 mpmc_stress 2>&1 | grep -E "^error|^test |panicked|enqueued|TIMEOUT|left|right" | cut -c1-200
