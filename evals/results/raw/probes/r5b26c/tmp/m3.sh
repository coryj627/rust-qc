cd /work/probe; export CARGO_TARGET_DIR=/target/probe
timeout 100 cargo test --offline --test panics -- --test-threads=1 2>&1 | grep -E "^test |result|panicked at /work" | grep -v "\.\.\. ok" | cut -c1-200
echo "=== miri panics"; 
for t in $(cargo test --offline --test panics -- --list 2>/dev/null | sed -n 's/: test$//p'); do
  r=$(timeout 60 cargo +nightly-2026-10-01 miri test --offline --test panics -- --exact $t 2>&1 | grep -E "^error: Undefined|^error: unsupported|test result|--> /work/crate|memory leaked" | cut -c1-170 | head -3 | tr '\n' ' ')
  echo "$t: $r"
done
echo "=== miri fuzz"; timeout 400 cargo +nightly-2026-10-01 miri test --offline --test fuzz -- --skip deque_contig --skip truncate --skip cstring 2>&1 | grep -E "^error|^test |test result|--> /work/crate" | cut -c1-200 | tail -20
