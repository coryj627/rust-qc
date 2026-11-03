"""Tests for check.sh, using stub cargo and rustup commands so no toolchain or network is needed.

Run from the repository root: python3 -I tests/test_check.py

The bash on PATH runs the script, so only a run on macOS covers its bash 3.2.
"""
import os
import stat
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).parent
CHECK = HERE.parent / ".agents" / "skills" / "rust-qc" / "scripts" / "check.sh"
CRATE = HERE / "fixtures" / "binonly"

STUB_CARGO = """#!/bin/sh
echo "auto_install=$RUSTUP_AUTO_INSTALL cargo $*" >> "$STUB_LOG"
case "$*" in
    --version) [ -z "$PINNED_TOOLCHAIN_MISSING" ] || exit 1 ;;
    "fmt --all --check")
        if [ -n "$NIGHTLY_OPTIONS" ]; then
            echo "Warning: unstable features are only available in nightly channel." >&2
            exit "${STABLE_FMT_STATUS:-1}"
        fi
        ;;
    clippy\\ --workspace*) [ -z "$CLIPPY_FAILS" ] || exit 1 ;;
esac
exit 0
"""

STUB_RUSTUP = """#!/bin/sh
echo "auto_install=$RUSTUP_AUTO_INSTALL rustup $* toolchain=$RUSTUP_TOOLCHAIN" >> "$STUB_LOG"
case "$*" in
    --version) echo "rustup ${STUB_RUSTUP_VERSION:-1.29.0} (28d1352db 2026-03-05)" ;;
    "toolchain list")
        echo "stable-aarch64-apple-darwin (active, default)"
        echo "$EXTRA_TOOLCHAIN"
        ;;
    "target list --installed") echo "x86_64-pc-windows-msvc" ;;
esac
exit 0
"""

failures = []


def expect(name, condition, detail):
    if not condition:
        failures.append(f"{name}: {detail}")


with tempfile.TemporaryDirectory() as tmp:
    log = Path(tmp) / "log"
    for name, body in (("cargo", STUB_CARGO), ("rustup", STUB_RUSTUP)):
        stub = Path(tmp) / name
        stub.write_text(body)
        stub.chmod(stub.stat().st_mode | stat.S_IEXEC)

    def run(*args, path=None, **stub_env):
        log.write_text("")
        env = {**os.environ, "PATH": path or f"{tmp}{os.pathsep}{os.environ['PATH']}",
               "STUB_LOG": str(log), **stub_env}
        env.pop("RUSTUP_AUTO_INSTALL", None)
        out = subprocess.run(["bash", str(CHECK), *args], capture_output=True, text=True,
                             env=env, check=False)
        return out.returncode, out.stdout + out.stderr, log.read_text()

    code, out, calls = run(str(CRATE))
    expect("passing crate", code == 0, f"exit {code}\n{out}")
    for step in ("format", "clippy", "rustdoc", "tests"):
        expect("passing crate", f"pass  {step}" in out, f"no pass line for {step}\n{out}")

    code, out, _ = run("--target", "x86_64-pc-windows-msvc", str(CRATE))
    expect("cross target", code == 0, f"exit {code}\n{out}")
    expect("cross target", "skip  tests" in out, f"tests were not skipped\n{out}")

    code, out, _ = run(str(CRATE), CLIPPY_FAILS="1")
    expect("failing step", code == 1, f"exit {code}, want 1\n{out}")
    expect("failing step", "pass  tests" in out, f"later steps did not run\n{out}")

    for name, args in (
        ("no arguments", ()),
        ("option but no crate root", ("--target", "x86_64-pc-windows-msvc")),
        ("unknown option", ("--bogus", str(CRATE))),
        ("missing manifest", (str(HERE),)),
        ("target not installed", ("--target", "no-such-triple", str(CRATE))),
    ):
        code, out, _ = run(*args)
        expect(name, code == 2, f"exit {code}, want 2\n{out}")

    code, out, calls = run(str(CRATE), PINNED_TOOLCHAIN_MISSING="1")
    expect("pinned toolchain missing", code == 2, f"exit {code}, want 2\n{out}")

    # A stable run that warns and also finds a diff must not fail the crate: stable's diffs are
    # not the standard for a project with nightly-only options.
    code, out, calls = run(str(CRATE), NIGHTLY_OPTIONS="1",
                           EXTRA_TOOLCHAIN="nightly-aarch64-apple-darwin")
    expect("nightly options", code == 0, f"exit {code}\n{out}")
    expect("nightly options", "cargo +nightly fmt --all --check" in calls, f"not used\n{calls}")

    for name, extra in (("nightly absent", ""),
                        ("only a dated nightly", "nightly-2026-01-01-aarch64-apple-darwin")):
        for status in ("0", "1"):
            code, out, calls = run(str(CRATE), NIGHTLY_OPTIONS="1", EXTRA_TOOLCHAIN=extra,
                                   STABLE_FMT_STATUS=status)
            label = f"{name}, stable fmt exit {status}"
            expect(label, code == 0, f"exit {code}\n{out}")
            expect(label, "skip  format" in out, f"format was not skipped\n{out}")
            # rustup downloads a toolchain named with `+` if it is missing.
            expect(label, "+nightly" not in calls, f"asked rustup for nightly\n{calls}")

    # rustup before 1.28 downloads a missing toolchain with no way to opt out, so the gate stops
    # before cargo runs. The versions compare as numbers, and an unreadable one counts as too old.
    for version, want in (("1.27.1", 2), ("1.9.0", 2), ("0.99.0", 2), ("unknown", 2),
                          ("1.28.0", 0), ("1.100.0", 0), ("2.0.0", 0)):
        code, out, calls = run(str(CRATE), STUB_RUSTUP_VERSION=version)
        expect(f"rustup {version}", code == want, f"exit {code}, want {want}\n{out}")
        if want == 2:
            expect(f"rustup {version}", " cargo " not in calls, f"ran cargo first\n{calls}")

    no_rustup = Path(tmp) / "no_rustup"
    no_rustup.mkdir()
    (no_rustup / "cargo").symlink_to(Path(tmp) / "cargo")
    code, out, _ = run(str(CRATE), path=os.pathsep.join((str(no_rustup), "/usr/bin", "/bin")))
    expect("cargo without rustup", code == 0, f"exit {code}\n{out}")

    code, out, calls = run(str(CRATE))
    # An old rustup's `--version` looks up the active toolchain and installs it if it is missing,
    # unless a toolchain is named for that one call.
    expect("rustup version query", "rustup --version toolchain=rust-qc-" in calls, f"{calls}")
    for line in calls.splitlines():
        expect("auto-install off", line.startswith("auto_install=0 "), f"{line}\n{out}")

print("\n".join(failures) or "all check.sh tests passed")
sys.exit(1 if failures else 0)
