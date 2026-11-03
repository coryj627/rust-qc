#!/bin/sh
# Reproducer for zip-2. Exits 0 only if the bug is demonstrated at the baseline.
# Usage: sb run zip sh /work/repro/zip-2/run.sh
set -e
export TMPDIR=/tmp
cd /work/repro/_crate
exec cargo test --locked --test zip_2_setuid -- --nocapture --test-threads=1
