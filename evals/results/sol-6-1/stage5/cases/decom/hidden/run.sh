#!/bin/sh
# Assessor-only runner. Never expose this directory or its outputs to treatment agents.
set -eu

if [ "$#" -ne 2 ]; then
    printf 'Usage: sh /work/prepared/hidden/run.sh CANDIDATE_WORKSPACE REPORT_DIRECTORY\n' >&2
    exit 2
fi

candidate=$(cd "$1" && pwd -P)
mkdir -p "$2"
report=$(cd "$2" && pwd -P)
case "$report/" in
    "$candidate/"*) printf 'Report directory must be outside the candidate workspace.\n' >&2; exit 2 ;;
esac
if [ ! -f "$candidate/alacritty_terminal/Cargo.toml" ]; then
    printf 'Expected an Alacritty workspace with alacritty_terminal/Cargo.toml.\n' >&2
    exit 2
fi
if [ -e "$candidate/alacritty_terminal/tests/decom_case_hidden.rs" ]; then
    printf 'Reserved hidden-test filename already exists; do not overwrite candidate files.\n' >&2
    exit 2
fi

export RUSTUP_AUTO_INSTALL=0
export CARGO_NET_OFFLINE=true
export CARGO_HOME=/work/.cargo-home
export TMPDIR=/work/tmp
mkdir -p "$TMPDIR"
run_dir=$(mktemp -d /work/tmp/decom-assess.XXXXXX)
trap 'rm -rf "$run_dir"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
export CARGO_TARGET_DIR="$run_dir/target"

# Capture the submitted diff BEFORE injecting the hidden test.
mkdir "$run_dir/baseline" "$run_dir/source"
tar -xzf /work/prepared/evidence/baseline-source.tar.gz -C "$run_dir/baseline"
cp -a "$candidate/." "$run_dir/source/"
set +e
diff -ruN "$run_dir/baseline" "$run_dir/source" > "$report/candidate.diff"
diff_status=$?
set -e
if [ "$diff_status" -gt 1 ]; then
    printf 'Could not compare the submitted source against the pristine baseline.\n' >&2
    exit 2
fi
cp /work/prepared/hidden/decom_case_hidden.rs "$run_dir/source/alacritty_terminal/tests/decom_case_hidden.rs"

{
    rustc +1.99.1 --version --verbose
    cargo +1.99.1 --version
    printf 'candidate=%s\n' "$candidate"
    printf 'package=alacritty_terminal\n'
    printf 'RUSTUP_AUTO_INSTALL=0 CARGO_NET_OFFLINE=true TMPDIR=/work/tmp\n'
    printf 'temporary_target=%s\n' "$CARGO_TARGET_DIR"
} > "$report/environment.txt"

printf 'check\texit_code\tlog\n' > "$report/status.tsv"
overall=0
run_check() {
    check_name=$1
    shift
    set +e
    "$@" > "$report/$check_name.log" 2>&1
    check_status=$?
    set -e
    printf '%s\t%s\t%s.log\n' "$check_name" "$check_status" "$check_name" >> "$report/status.tsv"
    printf '%s exit=%s\n' "$check_name" "$check_status"
    if [ "$check_status" -ne 0 ]; then
        overall=1
    fi
}

cd "$run_dir/source"
run_check hidden-compile-default cargo +1.99.1 test --locked --offline -p alacritty_terminal --test decom_case_hidden --no-run
run_check hidden-default cargo +1.99.1 test --locked --offline -p alacritty_terminal --test decom_case_hidden
run_check hidden-compile-no-default cargo +1.99.1 test --locked --offline -p alacritty_terminal --no-default-features --test decom_case_hidden --no-run
run_check hidden-no-default cargo +1.99.1 test --locked --offline -p alacritty_terminal --no-default-features --test decom_case_hidden
run_check existing-lib-default cargo +1.99.1 test --locked --offline -p alacritty_terminal --lib
run_check existing-ref-default cargo +1.99.1 test --locked --offline -p alacritty_terminal --test ref
run_check docs-default cargo +1.99.1 test --locked --offline -p alacritty_terminal --doc
# Discover candidate-added integration targets, so coverage credit is based on execution.
# Metadata resolves only the already locked/fetched packages; it builds no workspace member.
run_check candidate-target-inventory sh -c 'cargo +1.99.1 metadata --locked --offline --no-deps --format-version 1 > "$1"' sh "$report/metadata.json"
if [ -s "$report/metadata.json" ]; then
    python3 - "$report/metadata.json" > "$report/candidate-integration-targets.txt" <<'PY_TARGETS'
import json
import sys
from pathlib import Path
metadata = json.loads(Path(sys.argv[1]).read_text())
for package in metadata["packages"]:
    if package["name"] == "alacritty_terminal":
        for target in package["targets"]:
            if "test" in target["kind"] and target["name"] not in {"ref", "decom_case_hidden"}:
                print(target["name"])
PY_TARGETS
    integration_index=0
    while IFS= read -r integration_target; do
        integration_index=$((integration_index + 1))
        run_check "candidate-integration-$integration_index" cargo +1.99.1 test --locked --offline -p alacritty_terminal --test "$integration_target"
    done < "$report/candidate-integration-targets.txt"
fi
run_check existing-lib-no-default cargo +1.99.1 test --locked --offline -p alacritty_terminal --no-default-features --lib

printf 'Reports: %s\n' "$report"
exit "$overall"
