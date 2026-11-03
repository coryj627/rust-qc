#!/bin/sh
cd /work/probe
export CARGO_TARGET_DIR=/target/miri
RUST_BACKTRACE=0 MIRIFLAGS="-Zmiri-disable-isolation" timeout 400 cargo +nightly-2026-10-01 miri test --offline --release --test bal -- --test-threads=1 --skip indexmap_clear 2>&1 | grep -E "^(error|test )|Undefined Behavior|Data race|  --> |leaked" | cut -c1-220 | head -30
RUST_BACKTRACE=0 MIRIFLAGS="-Zmiri-disable-isolation" timeout 200 cargo +nightly-2026-10-01 miri test --offline --release --test bal -- --exact indexmap_clear_panic_then_insert --nocapture 2>&1 | grep -E "^(error|test |len|get)|Undefined Behavior|  --> " | cut -c1-220 | head -14
