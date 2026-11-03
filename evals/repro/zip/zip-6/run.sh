#!/bin/sh
# Reproducer for zip-6. Exits 0 only if the bug is demonstrated at the baseline.
# Usage: sb run zip sh /work/repro/zip-6/run.sh
set -e
export TMPDIR=/tmp
cd /work/repro/_crate
exec cargo test --locked --test zip_6_legacy_prealloc -- --nocapture --test-threads=1
