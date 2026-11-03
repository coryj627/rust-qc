#!/usr/bin/env python3
"""Append only completed, valid, fully graded Sol audits without rewriting YAML."""

import argparse
import csv
import json
from pathlib import Path

import yaml


def entries(root):
    with (root / "runs.tsv").open() as stream:
        runs = list(csv.DictReader(stream, delimiter="\t"))
    scores = json.loads((root / "key_scores.json").read_text())
    result = {}
    for run in runs:
        if (run["role"] != "audit" or run["validity"] != "complete" or
                run["status"] != "completed"):
            continue
        selected = [s for s in scores if str(s["stage"]) == run["stage"]
                    and s["project"] == run["project"]]
        if not selected or any(s["runs"].get(run["run"]) not in {"found", "partial", "missed"}
                               for s in selected):
            continue
        result.setdefault(run["project"], []).append({
            "run_id": run["run"], "stage": int(run["stage"]),
            "skill_version": run["skill_version"], "arm": run["arm"],
            "model": run["model"], "reasoning_effort": run["effort"],
            "time_budget_minutes": float(run["budget_minutes"]),
            "minutes_used": float(run["minutes"]),
            "report": f"results/sol-6-1/{run['report']}",
            "bugs_found": [s["bug"] for s in selected if s["runs"][run["run"]] == "found"],
            "bugs_partial": [s["bug"] for s in selected if s["runs"][run["run"]] == "partial"],
        })
    return result


def append(path, additions, apply=False):
    original = path.read_text()
    parsed = yaml.safe_load(original)
    known = {row.get("run_id") for row in parsed["runs"]}
    rows = [row for row in additions if row["run_id"] not in known]
    if not rows:
        return 0
    # JSON scalars/arrays are quoted YAML values. Preserve every existing byte.
    text = original + ("" if original.endswith("\n") else "\n")
    for row in rows:
        for index, (key, value) in enumerate(row.items()):
            text += f"{'  - ' if index == 0 else '    '}{key}: {json.dumps(value)}\n"
    expected = dict(parsed, runs=parsed["runs"] + rows)
    if yaml.safe_load(text) != expected or not text.startswith(original):
        raise ValueError("Append would alter existing key data or YAML values")
    if apply:
        path.write_text(text)
        if yaml.safe_load(path.read_text()) != expected:
            raise ValueError("Written YAML did not round-trip")
    return len(rows)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--apply", action="store_true")
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    keys = root.parent.parent / "keys"
    counts = {project: append(keys / f"{project}.yaml", rows, args.apply)
              for project, rows in entries(root).items()}
    print(json.dumps({"applied": args.apply, "appended_rows": counts}, indent=2))


if __name__ == "__main__":
    main()
