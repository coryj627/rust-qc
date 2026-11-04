#!/bin/sh
# Run Stage 3 for each project in turn and push after each.
# Usage: stage3_all.sh <clone> <results> <stop-at-usd> <project>...
# Stops before a project if the agents' summed cost has reached <stop-at-usd>.
# COMMIT_TRAILER holds the attribution lines for each commit message.
clone=$1; results=$2; stop=$3; shift 3
here=$(cd "$(dirname "$0")" && pwd)
all="$*"
for p in "$@"; do
    spent=$(python3 -I "$here/stage3_cost.py" $all)
    if python3 -I -c "import sys; sys.exit(0 if float('$spent') >= float('$stop') else 1)"; then
        echo "== stopping before $p: agents have spent $spent USD"; break
    fi
    python3 -I "$here/stage3.py" all "$clone" "$results" "$p" || { echo "== $p failed"; break; }
    python3 -I "$here/stage3_status.py" "$results" "$p" "$(python3 -I "$here/stage3_cost.py" $all)"
    cd "$clone" || exit 1
    python3 -I "$here/privacy_scan.py" "$results" > /dev/null || { echo "== privacy scan failed after $p"; break; }
    git add "$results" && git commit -q -m "Stage 3: $p

$COMMIT_TRAILER" && git pull -q --rebase origin eval/main-vs-v1.0.0 && git push -q origin eval/main-vs-v1.0.0
    echo "== $p pushed $(git rev-parse --short HEAD) $(date -u +%T)"
done
echo "== done $(date -u +%T)"
