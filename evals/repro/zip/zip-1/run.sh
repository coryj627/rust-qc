#!/bin/sh
# Reproducer for zip-1. Exits 0 only if the bug is demonstrated at the baseline.
# Usage: sb run zip sh /work/repro/zip-1/run.sh
set -e
export TMPDIR=/tmp
cd /work/repro/_crate
exec cargo test --locked --test zip_1_symlink_oom -- --nocapture --test-threads=1
