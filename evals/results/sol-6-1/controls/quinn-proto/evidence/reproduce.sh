#!/bin/sh
set -eu
# Recreates the locally tested scratch setup. Destination must not exist.
probe_scratch=${1:-/work/tmp/control-probe-reproduction}
probe_evidence=/work/prepared/evidence
if [ -e "$probe_scratch" ]; then
    echo "Destination already exists: $probe_scratch" >&2
    exit 1
fi
(cd /work/crate && sha256sum -c "$probe_evidence/baseline.sha256")
cp -a /work/crate "$probe_scratch"
while IFS= read -r probe_file; do
    cat "$probe_evidence/probes/$probe_file" >> "$probe_scratch/$probe_file"
done < "$probe_evidence/probe-files.txt"
cd "$probe_scratch"
RUSTUP_AUTO_INSTALL=0 cargo +1.99.1 test --locked -p quinn-proto negative_control_probes -- --nocapture
RUSTUP_AUTO_INSTALL=0 cargo +1.99.1 test --locked -p quinn-proto
