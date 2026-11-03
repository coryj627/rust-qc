#!/bin/sh
set -eu
export RUSTUP_AUTO_INSTALL=0
export CARGO_NET_OFFLINE=true
export CARGO_TARGET_DIR=/work/tmp/reference-check-target
runner=/work/prepared/hidden/run_api_tests.sh
"$runner" /work/reference > /work/prepared/reference-api.log 2>&1
cargo fmt --manifest-path /work/reference/Cargo.toml --check > /work/prepared/reference-fmt.log 2>&1
cargo clippy --manifest-path /work/reference/Cargo.toml --offline --all-targets -- -D warnings > /work/prepared/reference-clippy.log 2>&1
for variant in unicode boundary; do
    log="/work/prepared/negative-$variant.log"
    if "$runner" "/work/reference/negative_$variant" > "$log" 2>&1; then
        echo "ERROR: negative $variant unexpectedly passed" >&2
        exit 1
    else
        status=$?
        if [ "$status" -ne 101 ]; then
            echo "ERROR: negative $variant exited $status rather than cargo test failure" >&2
            exit 1
        fi
    fi
    python3 - "$variant" "$log" <<'PY'
import pathlib
import sys
variant, log_path = sys.argv[1:]
log = pathlib.Path(log_path).read_text()
expected = {
    'unicode': ['unicode_values_are_preserved', 'supported_escape_and_unicode_combinations'],
    'boundary': ['lf_crlf_mixed_endings_and_unterminated_final_line', 'cr_is_only_a_line_ending_when_immediately_before_lf'],
}[variant]
assert 'test result: FAILED.' in log, log
assert 'error[E' not in log, log
for name in expected:
    assert f'test {name} ... FAILED' in log, log
print(f'negative {variant}: correctly rejected; expected behavioral failures observed')
PY
done
printf '%s\n' 'reference: 24 API tests passed; rustfmt and clippy passed'
