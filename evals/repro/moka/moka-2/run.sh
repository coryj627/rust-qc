#!/bin/sh
. /work/repro/lib.sh
prepare moka-2
timeout 120 cargo run --offline --quiet --release
