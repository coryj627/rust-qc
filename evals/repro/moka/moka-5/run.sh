#!/bin/sh
. /work/repro/lib.sh
prepare moka-5
cargo run --offline --quiet --release
