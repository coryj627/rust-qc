#!/usr/bin/env python3
"""Publish completed control and assessment evidence with explicit allowlists."""

import argparse
import hashlib
import json
from pathlib import Path

from collect_grades import completed
from export_dataset import scrub, workspace_files


def copy_evidence(source, destination, scratch):
    files = {}
    for path, relative in workspace_files(source):
        data = path.read_bytes()
        if len(data) > 10_000_000 or data.startswith((b"\x7fELF", b"!<arch>\n", b"MZ")):
            raise ValueError(f"Unexpected binary or oversized evidence: {relative}")
        target = destination / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        try:
            target.write_text(scrub(data.decode("utf-8"), scratch))
        except UnicodeDecodeError:
            target.write_bytes(data)
        target.chmod(path.stat().st_mode & 0o777)
        files[str(relative)] = hashlib.sha256(target.read_bytes()).hexdigest()
    return files


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("scratch", type=Path)
    args = parser.parse_args()
    scratch = args.scratch.resolve()
    output = Path(__file__).resolve().parent.parent
    collected = []
    for row in json.loads((scratch / "controls-schedule.json").read_text()):
        run, project = row["run_id"], row["project"]
        if not completed(scratch, run):
            continue
        source = scratch / "audits/projects" / run / "prepared"
        controls = json.loads((source / "controls.json").read_text())
        decoy_ids = {c["id"] for c in controls["decoys"]}
        false_ids = {c["id"] for c in controls["false_findings"]}
        if decoy_ids != {"decoy-1", "decoy-2"} or false_ids != {f"false-{i}" for i in range(1, 6)}:
            raise ValueError(f"Wrong control IDs/counts for {project}")
        for entry in controls["decoys"] + controls["false_findings"]:
            refutation = entry["refutation"]
            if refutation["method"] not in {"ran", "read"} or not refutation["evidence"]:
                raise ValueError(f"Unconfirmed control in {project}")
        destination = output / "controls" / project
        hashes = copy_evidence(source, destination, scratch)
        record = {"author_run": run, "model": "gpt-6.1-sol", "effort": "high",
                  "audit_outputs_seen": False, "decoys": 2, "false_findings": 5,
                  "sha256": hashes}
        (destination / "preparation.json").write_text(json.dumps(record, indent=2) + "\n")
        collected.append(run)
    for schedule in scratch.glob("assessment-*.json"):
        row = json.loads(schedule.read_text())[0]
        run, case = row["run_id"], row["case"]
        if not completed(scratch, run):
            continue
        source = scratch / "audits/projects" / run / "results"
        if not (source / "assessment.json").is_file():
            raise ValueError(f"Missing assessment results for {case}")
        destination = output / "stage5/assessments" / case
        hashes = copy_evidence(source, destination, scratch)
        (destination / "provenance.json").write_text(json.dumps({
            "assessor_run": run, "model": "gpt-6.1-sol", "effort": "high", "sha256": hashes,
        }, indent=2) + "\n")
        collected.append(run)
    print(json.dumps({"collected_support": collected}, indent=2))


if __name__ == "__main__":
    main()
