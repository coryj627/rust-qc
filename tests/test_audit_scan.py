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
    return {(m.group(1), int(m.group(2))) for m in re.finditer(r"^(\S+) \S+:(\d+)$", out.stdout, re.M)}


def expected(fixture):
    lines = (HERE / "fixtures" / fixture / "src" / "lib.rs").read_text().splitlines()
    return {(m.group(1), i + 1) for i, l in enumerate(lines) for m in [re.search(r"EXPECT (\S+)", l)] if m}


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
