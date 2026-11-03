#!/bin/sh
# Reproducer for zip-4. Exits 0 only if the bug is demonstrated at the baseline.
# Usage: sb run zip sh /work/repro/zip-4/run.sh
set -e
export TMPDIR=/tmp
cd /work/repro/_crate
exec cargo test --locked --test zip_4_declared_size -- --nocapture --test-threads=1
