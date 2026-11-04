#!/bin/sh
set -eu
cd /work/tmp/git2-controls
RUSTUP_AUTO_INSTALL=0 cargo test --locked -p git2 --no-default-features --test controls_regression -- --nocapture --test-threads=1
