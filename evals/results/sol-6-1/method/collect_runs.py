#!/usr/bin/env python3
"""Publish assignments and actual run status without treating missing runs as zero."""

import argparse
import csv
import json
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("scratch", type=Path)
    args = parser.parse_args()
    scratch = args.scratch.resolve()
    output = Path(__file__).resolve().parent.parent
    rows, labels = {}, {}
    for mapping in (scratch / "grading-private").glob("*/map.json"):
        for label, run in json.loads(mapping.read_text())["reports"].items():
            labels[run] = label
    for stage in (3, 4, 5):
        schedule = scratch / f"stage{stage}-schedule.json"
        if schedule.exists():
            for row in json.loads(schedule.read_text()):
                rows[row["run_id"]] = dict(row, role="audit" if stage in (3, 4) else "treatment")
    for pattern in ("controls-schedule.json", "grading-*.json", "assessment-*.json",
                    "quality-*.json", "adjudication-*.json", "recheck-*.json"):
        for schedule in scratch.glob(pattern):
            for row in json.loads(schedule.read_text()):
                rows[row["run_id"]] = dict(row, stage=row.get("stage", "preparation-or-grading"))
    for metadata_path in (scratch / "private").glob("*/metadata.json"):
        run = metadata_path.parent.name
        meta = json.loads(metadata_path.read_text())
        if run not in rows:
            stage = 2 if run.startswith(("discover-", "skip-")) else "preparation-or-grading"
            if run.startswith("author-"):
                role = "case-author"
            elif run.startswith("control-"):
                role = "control-author"
            else:
                role = "discovery" if stage == 2 else "grader-or-assessor"
            rows[run] = {"run_id": run, "stage": stage, "role": role,
                         "project": "", "arm": "", "budget_minutes": meta["budget_seconds"] / 60}
        rows[run]["metadata"] = meta
        review = metadata_path.with_name("input_review.json")
        if review.exists():
            rows[run]["input_review"] = json.loads(review.read_text())
    blocks = {
        (str(row["stage"]), row.get("project")) for row in rows.values()
        if (row.get("metadata", {}).get("error") or {}).get("codexErrorInfo") == "cyberPolicy"
    }
    dispatch_reasons = {}
    for ledger in sorted(scratch.glob("dispatch-*.json"), key=lambda p: p.stat().st_mtime):
        for decision in json.loads(ledger.read_text()):
            if decision.get("reason"):
                dispatch_reasons[decision["run_id"]] = decision["reason"]
    fields = ["run", "stage", "role", "project", "arm", "letter", "skill_version",
              "requested_model", "requested_effort", "model", "effort", "budget_minutes", "minutes",
              "slot_wait_seconds", "status", "failure_kind", "killed_commands",
              "report", "validity", "absence_reason"]
    with (output / "runs.tsv").open("w", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=fields, delimiter="\t", lineterminator="\n")
        writer.writeheader()
        for run, row in rows.items():
            meta = row.get("metadata", {})
            invalid = row["role"] == "discovery" and not run.endswith("-v2")
            status = meta.get("status", "not_started")
            validity = "invalid-read-only-harness" if invalid else (
                "complete" if status == "completed" else "missing-or-in-progress"
            )
            if row.get("input_review", {}).get("valid") is False:
                validity = "invalid-other-source-skill"
            reason = ""
            if validity.startswith("invalid-"):
                reason = validity
            elif status == "not_started":
                reason = "project_provider_policy_block" if \
                    (str(row["stage"]), row.get("project")) in blocks else \
                    dispatch_reasons.get(run, "not_yet_dispatched")
            elif status != "completed":
                reason = (meta.get("error") or {}).get("codexErrorInfo") or status
            minutes = round(meta["active_seconds"] / 60, 3) if "active_seconds" in meta else ""
            report = ""
            if meta.get("report_collected"):
                report = f"{'invalid-attempts/' if invalid else ''}reports/{run}.md"
            writer.writerow({
                "run": run, "stage": row["stage"], "role": row["role"],
                "project": row.get("project", ""), "arm": row.get("arm", ""),
                "letter": labels.get(run, ""),
                "skill_version": meta.get("skill_version") or
                ("v1.0.0" if row.get("arm") == "with-skill" else "none"),
                "requested_model": "gpt-6.1-sol", "requested_effort": "high",
                "model": meta.get("model", ""),
                "effort": meta.get("effort", ""), "budget_minutes": row["budget_minutes"],
                "minutes": minutes,
                "slot_wait_seconds": meta.get("slot_wait_seconds", ""), "status": status,
                "failure_kind": (meta.get("error") or {}).get("codexErrorInfo", ""),
                "killed_commands": json.dumps(meta.get("command_timeouts", [])) if meta else "",
                "report": report,
                "validity": validity,
                "absence_reason": reason,
            })
    print(f"Recorded {len(rows)} planned/attempted runs; missing observations retain empty values")


if __name__ == "__main__":
    main()
