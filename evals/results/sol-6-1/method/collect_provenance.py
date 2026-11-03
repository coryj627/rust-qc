#!/usr/bin/env python3
"""Export explicit schedule, blinding and grader-input allowlists after grading."""

import argparse
import hashlib
import json
from pathlib import Path

from export_dataset import scrub

PLAN_PATTERNS = (
    "stage[345]-schedule.json", "controls-schedule.json", "assessment-*.json",
    "grading-*.json", "quality-*.json", "adjudication-*.json",
    "reconciliation-*.json", "recheck-s[34]-*.json", "dispatch-*.json",
    "execution-assessors.json", "recommendations-schedule.json",
)


def copy_text(source, target, scratch):
    if source.is_symlink() or not source.is_file():
        raise ValueError(f"Expected a regular provenance file: {source.name}")
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(scrub(source.read_text(), scratch))
    return hashlib.sha256(target.read_bytes()).hexdigest()


def collect(scratch, output):
    hashes = {}

    def save(source, relative):
        hashes[str(relative)] = copy_text(source, output / relative, scratch)

    for pattern in PLAN_PATTERNS:
        for source in sorted(scratch.glob(pattern)):
            save(source, Path("schedules") / source.name)
    selection = scratch / "recheck-selection.json"
    if selection.exists():
        save(selection, Path("grading/recheck-selection.json"))
    for private in sorted((scratch / "grading-private").iterdir()):
        if not private.is_dir() or private.is_symlink():
            raise ValueError("Unexpected private grading directory")
        cell = private.name
        mapping = json.loads((private / "map.json").read_text())
        for name in ("map.json", "redactions.json", "verifier-normalization.json",
                     "operator-input-integrity.json"):
            if (private / name).exists():
                save(private / name, Path("grading") / cell / name)
        if cell.startswith("s5-"):
            source = scratch / "audits/projects" / f"assess-{cell}"
            for label in mapping["reports"]:
                save(source / "reports" / f"{label}.md",
                     Path("grading") / cell / "reports" / f"{label}.md")
        else:
            source = scratch / "audits/projects" / f"key-{cell}-1"
            save(source / "key.json", Path("grading") / cell / "key.json")
            for label in mapping["reports"]:
                save(source / f"report-{label}.md",
                     Path("grading") / cell / f"report-{label}.md")
            save(scratch / "audits/projects" / f"verify-{cell}" / "report-passages.json",
                 Path("grading") / cell / "report-passages.json")
    record = {"allowlist": True, "sha256": hashes,
              "excluded": ["profiles", "authentication", "raw app-server events",
                           "reasoning", "server logs", "transport identifiers"]}
    (output / "grading").mkdir(exist_ok=True)
    (output / "grading/provenance.json").write_text(json.dumps(record, indent=2) + "\n")
    return len(hashes)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("scratch", type=Path)
    args = parser.parse_args()
    output = Path(__file__).resolve().parent.parent
    print(f"Exported {collect(args.scratch.resolve(), output)} provenance files")


if __name__ == "__main__":
    main()
