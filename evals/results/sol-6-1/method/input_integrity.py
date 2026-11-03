#!/usr/bin/env python3
"""Check assessor runtime assets against the case commit frozen before treatment."""

import hashlib
import json
import subprocess
from pathlib import Path

from export_dataset import workspace_files

CASE_FREEZE = "08dc878e24394976c0cfcbbe358a9f1cf53782da"


def verify(scratch, case):
    repository = Path(__file__).resolve().parents[4]
    root = Path(__file__).resolve().parent.parent
    source = root / "stage5/cases" / case
    prepared = scratch / "audits/projects" / f"assess-s5-{case}" / "prepared"
    names = [Path(name) for name in ("case.md", "rubric.md", "scoring.md")]
    names.extend(Path("hidden") / relative for _, relative in workspace_files(source / "hidden"))
    if case == "decom":
        names.extend(Path("evidence") / name for name in
                     ("baseline-source.tar.gz", "baseline-files.sha256"))
    hashes = {}
    for relative in names:
        committed_path = (source / relative).relative_to(repository)
        frozen = subprocess.check_output(["git", "show", f"{CASE_FREEZE}:{committed_path}"],
                                         cwd=repository)
        supplied = prepared / relative
        if (supplied.is_symlink() or
                not supplied.resolve().is_relative_to(prepared.parent.resolve()) or
                supplied.read_bytes() != frozen):
            raise ValueError(f"Assessment input changed from the frozen case: {case}/{relative}")
        hashes[str(relative)] = hashlib.sha256(frozen).hexdigest()
    assessor = prepared.parent
    gate_path = assessor / "gate/check.sh"
    if gate_path.is_symlink() or not gate_path.resolve().is_relative_to(assessor.resolve()):
        raise ValueError("Assessor gate escaped its workspace")
    gate = gate_path.read_bytes()
    if gate != (scratch / "skill-tree/.agents/skills/rust-qc/scripts/check.sh").read_bytes():
        raise ValueError("Assessor gate differs from the archived skill")
    record = {"case": case, "case_freeze_commit": CASE_FREEZE,
              "checked_by": "operator file comparison after assessment, before quality grading",
              "runtime_assets_match_frozen_commit": True, "sha256": hashes,
              "gate_matches_archived_skill": True,
              "gate_sha256": hashlib.sha256(gate).hexdigest(),
              "scope": "Case, rubric, scoring procedure, hidden tests/runners and baseline assets."
                       " This checks file integrity, not behavior or quality.",
              "withheld_author_material": "Author logs/reference implementations were not supplied."
                       " DECOM's full prepared.sha256 manifest also indexes those withheld logs;"
                       " its absence in the assessment is retained as a harness limitation."
                       " This separate comparison confirms the runtime assets actually supplied."}
    path = scratch / "grading-private" / f"s5-{case}" / "operator-input-integrity.json"
    path.write_text(json.dumps(record, indent=2) + "\n")
    return record
