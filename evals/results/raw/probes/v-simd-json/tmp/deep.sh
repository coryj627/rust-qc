cd /work/tmp/p
RUSTFLAGS="--cap-lints warn" cargo build --release --offline 2>&1 | grep -E "^error|could not" -A6
B=/target/release/p
$B sur 2>&1
for c in ub ub2 ub3; do echo "-- $c release"; $B $c 2>&1 | tail -4; echo "exit $?"; done
for k in owned borrowed de ign ownedobj encode tape; do for n in 5000 20000 50000 100000 1000000; do printf "%s %s: " $k $n; $B deep $k $n 2>&1 | tail -1 | cut -c1-100; done; done
RUSTFLAGS="--cap-lints warn" cargo build --offline 2>&1 | grep -E "^error|could not" -A6
D=/target/debug/p
for c in ub ub2 ub3; do echo "-- $c debug"; $D $c 2>&1 | tail -4; done
for n in 5000 10000 20000; do printf "debug de %s: " $n; $D deep de $n 2>&1 | tail -1 | cut -c1-100; done
