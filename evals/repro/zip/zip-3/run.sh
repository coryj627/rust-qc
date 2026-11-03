#!/bin/sh
# Reproducer for zip-3. Exits 0 only if the bug is demonstrated at the baseline.
# Usage: sb run zip sh /work/repro/zip-3/run.sh
set -e
export TMPDIR=/tmp
cd /work/repro/_crate
exec cargo test --locked --test zip_3_symlink_escape -- --nocapture --test-threads=1
