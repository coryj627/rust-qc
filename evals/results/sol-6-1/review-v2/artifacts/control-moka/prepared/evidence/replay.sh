#!/bin/sh
# Optional reproduction from the original baseline; this replay script is not the
# test invocation recorded in probe-run.log (which used moka-control-probe).
set -eu
replay_dir=/work/tmp/moka-control-replay
if [ -e "$replay_dir" ]; then
    echo "Replay directory already exists: $replay_dir" >&2
    exit 1
fi
cp -a /work/crate "$replay_dir"
cp /work/prepared/evidence/control_probes.rs "$replay_dir/src/prepared_control_probes.rs"
cat >> "$replay_dir/src/lib.rs" <<'RUST'

#[cfg(all(test, feature = "sync", feature = "future"))]
mod prepared_control_probes;
RUST
cd "$replay_dir"
cargo test --offline --locked --features sync,future --lib prepared_control_probes -- --nocapture --test-threads=1
