#!/bin/sh
# Set up, run and clean each project in turn. Usage: run_all_repros.sh <clone> <results> <project>...
clone=$1; results=$2; shift 2
here=$(dirname "$0")
for p in "$@"; do
    echo "== $p setup $(date -u +%T)"
    python3 -I "$here/run_repros.py" setup "$clone" "$p" > "$results/stage2/logs/setup-$p.txt" 2>&1 || { echo "setup failed: $p"; continue; }
    python3 -I "$here/run_repros.py" run "$clone" "$p" "$results/stage2/reproducers.tsv" "$results/stage2/logs"
    python3 -I "$here/run_repros.py" clean "$clone" "$p"
done
echo "== done $(date -u +%T)"
