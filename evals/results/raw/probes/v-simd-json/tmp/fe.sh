export RUSTFLAGS="--cap-lints warn"
F='^warning|^ +(-->|\||=)|^ *[0-9]+ \||^$|Compiling|Finished|Running|^note'
cd /work/tmp/q
for f in b128 bigf approx; do echo "=== probe $f"; cargo run --release --offline --features $f 2>&1 | grep -v -E "$F" | head -40; done
cd /work/crate
echo "=== check 128bit+approx"; CARGO_TARGET_DIR=/target/x cargo check --offline --lib --features 128bit,approx-number-parsing 2>&1 | grep -E "^error|-->.*approx|Finished" | head -6
echo "=== check nodefault approx"; CARGO_TARGET_DIR=/target/x cargo check --offline --lib --no-default-features --features approx-number-parsing 2>&1 | grep -E "^error|-->.*approx|Finished" | head -6
echo "=== check nodefault"; CARGO_TARGET_DIR=/target/x cargo check --offline --lib --no-default-features 2>&1 | grep -E "^error|Finished" | head -4
echo "=== test bigf"; CARGO_TARGET_DIR=/target/x cargo test --offline --lib --no-run --features big-int-as-float 2>&1 | grep -E "^error|-->.*correct|Finished" | head -6
echo "=== all-features"; CARGO_TARGET_DIR=/target/x cargo check --offline --lib --all-features 2>&1 | grep -E "^error|Finished" | head -6
echo "=== default w/o cap-lints"; RUSTFLAGS= CARGO_TARGET_DIR=/target/y cargo check --offline --lib 2>&1 | grep -E "^error|Finished" | head -5
grep -rn '"-0"\|-0"' src/tests src/numberparse/correct.rs src/tests.rs | head -5
cd /work/tmp/pn; echo "=== native route"; cargo run --offline 2>&1 | grep -v -E "$F" | head -8
