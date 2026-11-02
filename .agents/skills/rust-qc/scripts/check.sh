#!/usr/bin/env bash
# Runs the verification gate for a Rust crate or workspace.
#
# Usage: check.sh [--audit] [--target TRIPLE] [--features LIST] <crate-root>
#
#   --audit           Also run an advisory clippy pass with the lints that
#                     enforce this skill's documentation, unsafe, and panic
#                     rules. Its warnings are findings to report; they do not
#                     change the exit status.
#   --target TRIPLE   Lint and document for TRIPLE (for example
#                     x86_64-pc-windows-msvc) so code behind #[cfg(...)] for
#                     that platform is checked. Tests are skipped, because a
#                     cross target cannot run on this host.
#   --features LIST   Use these features instead of --all-features, for
#                     crates whose features are mutually exclusive.
#
# When rustfmt.toml uses nightly-only options, the format step uses nightly
# rustfmt, or is skipped with a reason if nightly rustfmt is not installed.
#
# The gate steps always all run, even after an earlier failure, so one
# invocation reports the full picture. A missing tool or manifest stops the
# script immediately, because results without it would be misleading.
#
# Exit status: 0 if every gate step passed, 1 if any failed, 2 on a
# precondition failure.

set -uo pipefail

audit=0
target=""
features=(--all-features)
while [[ $# -gt 1 ]]; do
    case "$1" in
        --audit) audit=1; shift ;;
        --target) target="$2"; shift 2 ;;
        --features) features=(--features "$2"); shift 2 ;;
        *) echo "error: unknown option $1" >&2; exit 2 ;;
    esac
done
root="${1:?usage: check.sh [--audit] [--target TRIPLE] [--features LIST] <crate-root>}"

if [[ ! -f "$root/Cargo.toml" ]]; then
    echo "error: no Cargo.toml in $root" >&2
    exit 2
fi
if ! command -v cargo >/dev/null 2>&1; then
    echo "error: cargo is not installed or not on PATH" >&2
    exit 2
fi
if ! cargo fmt --version >/dev/null 2>&1; then
    echo "error: rustfmt is not installed (rustup component add rustfmt)" >&2
    exit 2
fi
if ! cargo clippy --version >/dev/null 2>&1; then
    echo "error: clippy is not installed (rustup component add clippy)" >&2
    exit 2
fi

cd "$root" || exit 2

target_args=()
if [[ -n "$target" ]]; then
    if ! rustup target list --installed 2>/dev/null | grep -qx "$target"; then
        echo "error: target $target is not installed (rustup target add $target)" >&2
        exit 2
    fi
    target_args=(--target "$target")
fi

failed=0
results=()

run_step() {
    local name="$1"
    shift
    echo
    echo "== $name"
    if "$@"; then
        results+=("pass  $name")
    else
        results+=("FAIL  $name")
        failed=1
    fi
}

# A rustfmt.toml with nightly-only options formats differently on stable, so
# stable's diffs are not the project's standard. Use nightly rustfmt, as the
# project's own CI must, or skip the step and say why.
if cargo fmt --all --check 2>&1 >/dev/null | grep -q 'only available in nightly channel'; then
    if cargo +nightly fmt --version >/dev/null 2>&1; then
        run_step "format (nightly rustfmt, required by rustfmt.toml)" cargo +nightly fmt --all --check
    else
        results+=("skip  format (rustfmt.toml uses nightly-only options and nightly rustfmt is not installed)")
    fi
else
    run_step "format" cargo fmt --all --check
fi
run_step "clippy" cargo clippy --workspace --all-targets "${features[@]}" "${target_args[@]}" -- -D warnings
run_step "rustdoc" env RUSTDOCFLAGS="-D warnings" \
    cargo doc --workspace --no-deps --document-private-items "${features[@]}" "${target_args[@]}"
if [[ -z "$target" ]]; then
    run_step "tests" cargo test --workspace "${features[@]}"
else
    results+=("skip  tests (cross target $target cannot run here)")
fi

if [[ "$audit" == 1 ]]; then
    # Library and binary targets only, so #[cfg(test)] code is not compiled
    # and unwrap in tests is not reported.
    echo
    echo "== audit lints (advisory)"
    cargo clippy --workspace --lib --bins "${features[@]}" "${target_args[@]}" -- \
        -W missing_docs \
        -W missing_debug_implementations \
        -W clippy::missing_errors_doc \
        -W clippy::missing_panics_doc \
        -W clippy::undocumented_unsafe_blocks \
        -W clippy::unwrap_used \
        -W clippy::allow_attributes_without_reason \
        -W clippy::dbg_macro \
        -W clippy::todo \
        -W clippy::unimplemented \
        -W clippy::wildcard_imports \
        -W clippy::missing_assert_message \
        -W clippy::map_err_ignore \
        -W clippy::too_long_first_doc_paragraph
    results+=("done  audit lints (advisory; warnings above are findings)")
fi

echo
echo "== Summary"
printf '%s\n' "${results[@]}"

if grep -rqE '\bunsafe\b' --include='*.rs' --exclude-dir=target . 2>/dev/null; then
    echo
    echo "note: unsafe code present; also run 'cargo +nightly miri test'."
fi

exit "$failed"
