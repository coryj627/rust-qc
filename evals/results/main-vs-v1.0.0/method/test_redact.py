#!/usr/bin/env python3
"""Check redact.py on one fixture report per arm. Prints one line per check; exits 1 if any fails.

Usage: test_redact.py
"""

import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent))
from redact import redact

HERE = pathlib.Path(__file__).parent / "fixtures"
FAILED = 0


def check(name, ok):
    global FAILED
    FAILED += not ok
    print(f"{'pass' if ok else 'FAIL'}  {name}")


for arm, run_id in (("v1.0.0", "rf00000"), ("main", "rf00001")):
    source = (HERE / f"report-{arm}.md").read_text()
    out, dropped = redact(source, run_id)
    (HERE / f"report-{arm}.redacted.md").write_text(out)
    (HERE / f"report-{arm}.dropped-lines.txt").write_text(
        "".join(f"{r}\t{line}\n" for r, line in dropped)
    )
    headings = re.findall(r"^### +(.*)$", source, re.MULTILINE)
    check(
        f"{arm}: all {len(headings)} finding headings survive",
        all(f"**{h}**" in out for h in headings),
    )
    check(
        f"{arm}: no field labels left",
        not re.search(r"^(Location|Problem|Cause|Evidence[^:]*|Fix):", out, re.MULTILINE),
    )
    check(
        f"{arm}: no QA hygiene or Unverified wording", not re.search(r"QA hygiene|Unverified", out)
    )
    check(f"{arm}: workspace path replaced", "/srv/rqc" not in out and run_id not in out)
    check(
        f"{arm}: no skill, gate or brief mention",
        not re.search(r"rust-qc|\bskill\b|\bgate\b|\bbrief\b", out, re.IGNORECASE),
    )
    check(
        f"{arm}: verification section dropped and logged",
        "Verification" not in out and any(r == "verification section" for r, _ in dropped),
    )
    nonempty = [line for line in source.splitlines() if line.strip()]
    written = [line for line in out.splitlines() if line.strip()]
    check(
        f"{arm}: each of {len(nonempty)} source lines is written or logged as dropped",
        len(written) + len(dropped) == len(nonempty),
    )
sys.exit(1 if FAILED else 0)
