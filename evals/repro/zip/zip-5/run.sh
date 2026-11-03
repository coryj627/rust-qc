#!/bin/sh
# Reproducer for zip-5. Exits 0 only if the bug is demonstrated at the baseline.
# Usage: sb run zip sh /work/repro/zip-5/run.sh
set -e
export TMPDIR=/tmp
cd /work/repro/_crate
exec cargo test --locked --test zip_5_empty_cd_offset -- --nocapture --test-threads=1
