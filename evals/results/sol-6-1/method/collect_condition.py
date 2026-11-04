#!/usr/bin/env python3
"""Collect only a named condition's scheduled runs, preserving missing outcomes."""

import argparse
import collections
import csv
import hashlib
import json
import re
from pathlib import Path

from check_runs import check
from export_dataset import export, export_artifacts
from host_builds import violations


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("scratch", type=Path)
    parser.add_argument("schedule", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    scratch, output = args.scratch.resolve(), args.output.resolve()
    schedule = json.loads(args.schedule.read_text())
    seen = set()
    for row in schedule:
        if (not re.fullmatch(r"[a-z0-9-]+", row["run_id"])
                or not re.fullmatch(r"[a-z0-9-]+", row["project"])
                or row["run_id"] in seen):
            raise ValueError("Invalid or duplicate scheduled identifier")
        seen.add(row["run_id"])
    output.mkdir(parents=True, exist_ok=True)
    index_path = output / "export-index.json"
    index = json.loads(index_path.read_text()) if index_path.exists() else {}
    records, invalid = [], []
    for row in schedule:
        run = row["run_id"]
        private = scratch / "private" / run
        metadata_path = private / "metadata.json"
        try:
            metadata = json.loads(metadata_path.read_text()) if metadata_path.exists() else {}
        except json.JSONDecodeError:
            metadata = {"status": "metadata_snapshot_in_progress"}
        status = metadata.get("status", "not_started")
        ready = (status not in {"starting", "running", "metadata_snapshot_in_progress",
                                "not_started"}
                 and (private / "report.md").exists()
                 and not (private / "profile/auth.json").exists())
        validity = "missing-or-in-progress"
        if ready:
            checks = check(private)
            host = violations(private)
            validity = "complete" if status == "completed" and not checks["flags"] and not host \
                else "invalid" if status == "completed" else "incomplete"
            if validity == "invalid":
                invalid.append(run)
            inputs = [metadata_path, private / "report.md", private / "input_review.json"]
            fingerprint = hashlib.sha256(b"".join(
                p.read_bytes() for p in inputs if p.exists()
            )).hexdigest()
            if index.get(run, {}).get("fingerprint") != fingerprint:
                item = export(scratch, run, output, validity)
                template = scratch / "audits/projects" / f"tmpl-{row['project']}"
                item["artifacts"] = export_artifacts(
                    scratch, run, output, template if template.exists() else None,
                )
                index[run] = {"fingerprint": fingerprint, **item}
        records.append({
            "run": run, "condition": row.get("condition", ""),
            "stage": row["stage"], "role": row.get("role", "audit"),
            "project": row["project"], "arm": row["arm"],
            "skill_version": "v1.0.0" if row.get("skill") else "",
            "requested_model": "gpt-6.1-sol", "requested_effort": "high",
            "model": metadata.get("model", ""), "effort": metadata.get("effort", ""),
            "budget_minutes": row["budget_minutes"],
            "minutes": round(metadata["active_seconds"] / 60, 3)
            if "active_seconds" in metadata else "",
            "slot_wait_seconds": metadata.get("slot_wait_seconds", ""),
            "status": status, "failure_kind": (metadata.get("error") or {}).get(
                "codexErrorInfo", ""),
            "killed_commands": len(metadata.get("command_timeouts", [])) if metadata else "",
            "report": f"reports/{run}.md" if ready else "",
            "validity": validity,
        })
    if records:
        with (output / "runs.tsv").open("w") as stream:
            writer = csv.DictWriter(stream, fieldnames=list(records[0]), delimiter="\t")
            writer.writeheader()
            writer.writerows(records)
    index_path.write_text(json.dumps(index, indent=2) + "\n")
    progress = {"planned": len(records), "status_counts": dict(collections.Counter(
        r["status"] for r in records)), "validity_counts": dict(collections.Counter(
        r["validity"] for r in records)), "invalid_completed_runs": invalid,
        "provider_blocks": [r["run"] for r in records if r["failure_kind"] == "cyberPolicy"]}
    (output / "progress.json").write_text(json.dumps(progress, indent=2) + "\n")
    print(json.dumps(progress, indent=2))
    raise SystemExit(int(bool(invalid)))


if __name__ == "__main__":
    main()
