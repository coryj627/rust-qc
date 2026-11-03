cd /work/probe
export TMPDIR=/tmp
cargo build --offline --bin probe 2>&1 | grep -E "^error|^ +--> src|Finished" | head
B=/target/debug/probe
for w in add update remove; do echo "=== index_cb $w"; $B index_cb_empty_pathspec $w 2>&1 | grep -v "^note\|^stack\|^ *[0-9]*: \|^ *at " | cut -c1-200 | head -6; done
sh run_all.sh $B fetchhead_forms stash_no_message
cargo build --offline --release --bin probe 2>&1 | grep -E "^error|Finished"
for w in add; do echo "=== release index_cb $w"; /target/release/probe index_cb_empty_pathspec $w 2>&1 | cut -c1-200 | head -3; echo "rc=$?"; done
(export CFLAGS="-fsanitize=address -fno-omit-frame-pointer" CC=clang RUSTFLAGS="-Zsanitizer=address"
cargo +nightly-2026-10-01 build --offline --bin probe --target aarch64-unknown-linux-gnu --target-dir /target/asan 2>&1 | grep -E "^error|Finished|could not"
export ASAN_OPTIONS=detect_leaks=0
echo "=== asan diff_stress"; timeout 200 /target/asan/aarch64-unknown-linux-gnu/debug/probe diff_stress > /tmp/ds 2>&1; echo rc=$?; python3 /work/tmp/sym.py /target/asan/aarch64-unknown-linux-gnu/debug/probe /tmp/ds >/dev/null 2>&1; grep -E "ERROR: AddressSanitizer|^READ|^WRITE|^round|panicked" /tmp/ds | cut -c1-160 | head; )
echo "=== plain diff_stress"; timeout 120 $B diff_stress 2>&1 | tail -3 | cut -c1-200; echo rc=$?
