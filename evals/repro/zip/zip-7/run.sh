#!/bin/sh
# Reproducer for zip-7. Exits 0 only if the bug is demonstrated at the baseline.
# Usage: sb run zip sh /work/repro/zip-7/run.sh
set -e
export TMPDIR=/tmp
cd /work/repro/_crate
exec cargo test --locked --test zip_7_shrink_8191 -- --nocapture --test-threads=1
