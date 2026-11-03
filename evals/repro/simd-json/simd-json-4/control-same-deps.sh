#!/bin/sh
# Same tag, same halfbrown 0.3.0, but with the baseline's value-no-dup-keys feature (uses insert, not
# insert_nocheck). Shows the cause without the halfbrown 0.4 bump in the fix commit. Expected exit 1.
export RUSTFLAGS="--cap-lints allow" CARGO_TARGET_DIR=/target/nodup
cd /work/repro/simd-json-4 || exit 2
cargo run --offline -q --release --features simd-json/value-no-dup-keys
echo "exit=$?"
