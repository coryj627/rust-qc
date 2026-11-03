cd /work/tmp/probe
cargo test --offline --test more -- --nocapture --test-threads=1 --skip mpmc_stress 2>&1 | grep -E "^error|^test |panicked|assertion|lost-key|get=|reinsert|hint|len |precondition|left|right|E0" | cut -c1-220
echo "== serialize VecView compile check"
sed -e 's/#\[cfg(feature = "x")\] ser(v.as_view());/ser(v.as_view());/' tests/more.rs > tests/more2.rs
cargo check --offline --test more2 2>&1 | grep -E "^error" | head -3
rm -f tests/more2.rs
echo "== histbuf N=0"
mkdir -p tests; cat > tests/h0.rs <<'X'
use heapless::HistoryBuf;
#[test] fn h0() { let x: HistoryBuf<u8, 0> = HistoryBuf::new_with(1); println!("{}", x.len()); let _ = x.recent(); }
X
cargo test --offline --test h0 -- --nocapture 2>&1 | grep -E "^error|^test |panicked|overflow|out of" | cut -c1-200; rm -f tests/h0.rs
echo "== pool race miri"
MIRIFLAGS="-Zmiri-disable-isolation -Zmiri-many-seeds=0..24 -Zmiri-tree-borrows -Zmiri-ignore-leaks -Zmiri-permissive-provenance" timeout 240 cargo +nightly-2026-10-01 miri test --offline --test ub -- --exact pool_race 2>&1 | grep -E "Undefined Behavior|Data race|test result: F|^error" | sort | uniq -c | cut -c1-260 | head -8
