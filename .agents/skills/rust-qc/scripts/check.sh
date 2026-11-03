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
# invocation reports the full picture. A missing tool, toolchain, or manifest
# stops the script immediately, because results without it would be misleading.
# The script never installs a toolchain.
#
# Exit status: 0 if every gate step passed, 1 if any failed, 2 on a
# precondition failure.

set -uo pipefail

# rustup otherwise downloads any missing toolchain a command asks for, whether
# through `+nightly` or the project's rust-toolchain.toml.
export RUSTUP_AUTO_INSTALL=0

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
if [[ $# -ne 1 ]]; then
    echo "usage: check.sh [--audit] [--target TRIPLE] [--features LIST] <crate-root>" >&2
    exit 2
fi
root="$1"

if [[ ! -f "$root/Cargo.toml" ]]; then
    echo "error: no Cargo.toml in $root" >&2
    exit 2
fi
if ! command -v cargo >/dev/null 2>&1; then
    echo "error: cargo is not installed or not on PATH" >&2
    exit 2
fi

# rustup before 1.28 ignores RUSTUP_AUTO_INSTALL, so it could download a missing
# toolchain. Even its `--version` looks up the active toolchain, so this names a
# custom toolchain that does not exist, which fails that lookup without a download.
if command -v rustup >/dev/null 2>&1; then
    rustup_version="$(RUSTUP_TOOLCHAIN=rust-qc-no-such-toolchain rustup --version 2>/dev/null)"
    version_pattern='^rustup ([0-9]+)\.([0-9]+)'
    rustup_major=0
    rustup_minor=0
    if [[ "$rustup_version" =~ $version_pattern ]]; then
        rustup_major="${BASH_REMATCH[1]}"
        rustup_minor="${BASH_REMATCH[2]}"
    fi
    if (( rustup_major < 1 || (rustup_major == 1 && rustup_minor < 28) )); then
        echo "error: rustup 1.28 or newer is required; older versions can download toolchains with no way to turn that off" >&2
        exit 2
    fi
fi

# The checks below run from the crate root so they see its pinned toolchain.
cd "$root" || exit 2

if ! cargo --version >/dev/null 2>&1; then
    echo "error: cargo cannot run in $root; is the toolchain its rust-toolchain.toml names installed?" >&2
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

# Never empty: bash 3.2, the macOS default, treats expanding an empty array as
# an unbound variable under `set -u`. Pipelines below send grep's output to
# /dev/null instead of using -q, so an early exit cannot fail them under
# pipefail.
build_args=("${features[@]}")
if [[ -n "$target" ]]; then
    if ! rustup target list --installed 2>/dev/null | grep -x "$target" >/dev/null; then
        echo "error: target $target is not installed (rustup target add $target)" >&2
        exit 2
    fi
    build_args+=(--target "$target")
fi

# Asks rustup first, so `+nightly` is only used when nightly is already there. A
# dated nightly does not answer to `+nightly`.
nightly_rustfmt_installed() {
    rustup toolchain list 2>/dev/null | grep -E '^nightly-[^0-9]' >/dev/null &&
        cargo +nightly fmt --version >/dev/null 2>&1
}

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
# project's own CI must, or skip the step and say why. The warning is captured,
# not piped: stable's diff exits nonzero, which would fail a pipeline under
# pipefail even though the warning matched.
stable_fmt_stderr="$(cargo fmt --all --check 2>&1 >/dev/null)"
if [[ "$stable_fmt_stderr" == *'only available in nightly channel'* ]]; then
    if nightly_rustfmt_installed; then
        run_step "format (nightly rustfmt, required by rustfmt.toml)" cargo +nightly fmt --all --check
    else
        results+=("skip  format (rustfmt.toml uses nightly-only options and nightly rustfmt is not installed)")
    fi
else
    run_step "format" cargo fmt --all --check
fi
run_step "clippy" cargo clippy --workspace --all-targets "${build_args[@]}" -- -D warnings
run_step "rustdoc" env RUSTDOCFLAGS="-D warnings" \
    cargo doc --workspace --no-deps --document-private-items "${build_args[@]}"
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
    cargo clippy --workspace --lib --bins "${build_args[@]}" -- \
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
