#!/usr/bin/env python3
"""Provision only the selected submissions and their assessment evidence."""

import json
import shutil
import subprocess
from pathlib import Path

from prepare_audits import CLOCK


def provision(scratch, case, run_id, labels):
    source = scratch / "audits/projects" / f"assess-s5-{case}"
    work = scratch / "audits/projects" / run_id
    work.mkdir()
    for folder in ("tmp", "candidates", "reports", "results"):
        (work / folder).mkdir()
    for label in labels:
        subprocess.run(["/bin/cp", "-cR", str(source / "candidates" / label),
                        str(work / "candidates" / label)], check=True)
        shutil.copyfile(source / "reports" / f"{label}.md", work / "reports" / f"{label}.md")
        if (source / "results" / label).exists():
            subprocess.run(["/bin/cp", "-cR", str(source / "results" / label),
                            str(work / "results" / label)], check=True)
    assessment = json.loads((source / "results/assessment.json").read_text())
    selected = {"case": assessment["case"],
                "candidates": {label: assessment["candidates"][label] for label in labels}}
    (work / "results/assessment.json").write_text(json.dumps(selected, indent=2) + "\n")
    for name in ("prepared", "baseline", ".cargo-home"):
        if (source / name).exists():
            subprocess.run(["/bin/cp", "-cR", str(source / name), str(work / name)], check=True)
    structures = json.loads((Path(__file__).resolve().parent.parent /
                             "stage5/rubric-structure.json").read_text())
    (work / "rubric-structure.json").write_text(json.dumps(structures[case], indent=2) + "\n")
    integrity = scratch / "grading-private" / f"s5-{case}" / "operator-input-integrity.json"
    if integrity.exists():
        shutil.copyfile(integrity, work / "input-integrity.json")
    (work / "clock").write_text(CLOCK)
    (work / "clock").chmod(0o755)
    return work
