#!/bin/sh
# Reruns repro/<id>/run.sh against fixed/<id>: baseline with the fix commit's src/ diff applied.
# Expect a non-zero exit from run.sh, meaning the bug is no longer demonstrated.
id=$1
C=/work/control/$id
mkdir -p "$C/src"
cp /work/repro/$id/Cargo.lock /work/repro/$id/run.sh "$C/"
cp /work/repro/$id/src/main.rs "$C/src/main.rs"
[ -f /work/repro/$id/control_main.rs ] && cp /work/repro/$id/control_main.rs "$C/src/main.rs"
sed "s#\.\./\.\./base#../../fixed/$id#" /work/repro/$id/Cargo.toml > "$C/Cargo.toml"
REPRO_DIR=$C sh "$C/run.sh"
echo "[control $id: run.sh exit=$?]"
