"""Fixture tests for audit_scan.py. Run from the repository root: python3 -I tests/test_audit_scan.py"""
import re
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).parent
SCAN = HERE.parent / ".agents" / "skills" / "rust-qc" / "scripts" / "audit_scan.py"


def scan(fixture):
    out = subprocess.run([sys.executable, "-I", str(SCAN), str(HERE / "fixtures" / fixture)],
                         capture_output=True, text=True, check=False)
    if out.returncode != 0:
        sys.exit(f"{fixture}: scan exited {out.returncode}\n{out.stderr}")
    return {(m.group(1), m.group(2), int(m.group(3)))
            for m in re.finditer(r"^(\S+) (\S+):(\d+)$", out.stdout, re.M)}


def expected(fixture):
    root = HERE / "fixtures" / fixture
    return {(m.group(1), path.relative_to(root).as_posix(), i + 1)
            for path in (root / "src").rglob("*.rs")
            for i, line in enumerate(path.read_text().splitlines())
            for m in [re.search(r"EXPECT (\S+)", line)] if m}


failures = []
got, want = scan("flagged"), expected("flagged")
if got != want:
    failures.append(f"flagged: missing {sorted(want - got)}, unexpected {sorted(got - want)}")
for fixture in ("clean", "binonly"):
    got = scan(fixture)
    if got:
        failures.append(f"{fixture}: unexpected {sorted(got)}")
print("\n".join(failures) or "all scanner fixture tests passed")
sys.exit(1 if failures else 0)
