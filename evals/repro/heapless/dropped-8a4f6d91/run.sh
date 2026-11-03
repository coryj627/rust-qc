#!/bin/sh
cd /work/repro/dropped-8a4f6d91 || exit 2
timeout 60 cargo run -q; echo "exit $?"
