#!/bin/sh
set -eu
cd /work/tmp/control-crate
RUSTUP_AUTO_INSTALL=0 CARGO_NET_OFFLINE=true RUSTFLAGS="--cap-lints warn" cargo test --locked --test negative_controls -- --nocapture --test-threads=1
