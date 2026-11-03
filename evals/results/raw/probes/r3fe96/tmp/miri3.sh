#!/bin/sh
cd /work/probe
export CARGO_TARGET_DIR=/target/miri
RUST_BACKTRACE=0 MIRIFLAGS="-Zmiri-disable-isolation -Zmiri-preemption-rate=0.1 -Zmiri-many-seeds=0..24" timeout 600 cargo +nightly-2026-10-01 miri test --offline --release --features pool --test pool -- --nocapture 2>&1 | grep -E "^(error|test |  *--> )|Undefined Behavior|Data race|Trying seed|seed|integer_atomics" | cut -c1-240 | head -30
